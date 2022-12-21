use std::{env, fs, path::PathBuf};

use super::{
    EnvInfo, GenerateKeypair, ENV_CONFIG_HOST, ENV_KEYPAIR_BIN, ENV_MAX_COPIES, ENV_NET_ID, ENV_OUI,
};
use crate::{hex_field, Msg, PrettyJson, Result};
use anyhow::Context;
use dialoguer::Input;
use helium_crypto::Keypair;
use rand::rngs::OsRng;
use serde_json::json;

pub async fn env_init() -> Result<Msg> {
    println!("----- Leave blank to ignore...");
    let config_host: String = Input::new()
        .with_prompt("Config Service Host")
        .allow_empty(true)
        .interact()?;
    let keypair_path: String = Input::<String>::new()
        .with_prompt("Keypair Location")
        .with_initial_text("./keypair.bin")
        .allow_empty(true)
        .interact()?;
    println!("----- Enter all zeros to ignore...");
    let net_id = Input::<hex_field::HexNetID>::new()
        .with_prompt("Net ID")
        .with_initial_text("000000")
        .interact()?;
    println!("----- Enter zero to ignore...");
    let oui: u64 = Input::new()
        .with_prompt("Assigned OUI")
        .with_initial_text("0")
        .allow_empty(true)
        .interact()?;
    let max_copies: u32 = Input::new()
        .with_prompt("Default Max Copies")
        .allow_empty(true)
        .with_initial_text("15")
        .interact()?;

    let mut report = vec![
        "".to_string(),
        "Put these in your environment".to_string(),
        "------------------------------------".to_string(),
    ];
    if !config_host.is_empty() {
        report.push(format!("{ENV_CONFIG_HOST}={config_host}"));
    }
    if !keypair_path.is_empty() {
        report.push(format!("{ENV_KEYPAIR_BIN}={keypair_path}"))
    }
    if net_id != hex_field::net_id(0) {
        report.push(format!("{ENV_NET_ID}={net_id}"));
    }
    if oui != 0 {
        report.push(format!("{ENV_OUI}={oui}"));
    }
    if max_copies != 0 {
        report.push(format!("{ENV_MAX_COPIES}={max_copies}"));
    }

    Msg::ok(report.join("\n"))
}

pub fn env_info(args: EnvInfo) -> Result<Msg> {
    let env_keypair = env::var(ENV_KEYPAIR_BIN).ok().map(|i| i.into());
    let (env_keypair_location, env_public_key) = get_keypair(env_keypair);
    let (arg_keypair_location, arg_public_key) = get_keypair(args.keypair);

    let output = json!({
        "environment": {
            ENV_CONFIG_HOST: env::var(ENV_CONFIG_HOST).unwrap_or_else(|_| "unset".into()),
            ENV_NET_ID:  env::var(ENV_NET_ID).unwrap_or_else(|_| "unset".into()),
            ENV_OUI:  env::var(ENV_OUI).unwrap_or_else(|_| "unset".into()),
            ENV_MAX_COPIES: env::var(ENV_MAX_COPIES).unwrap_or_else(|_| "unset".into()),
            ENV_KEYPAIR_BIN:  env_keypair_location,
            "public_key_from_keypair": env_public_key,
        },
        "arguments": {
            "config_host": args.config_host,
            "net_id": args.net_id,
            "oui": args.oui,
            "max_copies": args.max_copies,
            "keypair": arg_keypair_location,
            "public_key_from_keypair": arg_public_key
        }
    });
    Msg::ok(output.pretty_json()?)
}

pub fn generate_keypair(args: GenerateKeypair) -> Result<Msg> {
    let key = helium_crypto::Keypair::generate(
        helium_crypto::KeyTag {
            network: helium_crypto::Network::MainNet,
            key_type: helium_crypto::KeyType::Ed25519,
        },
        &mut OsRng,
    );
    if let Some(parent) = args.out_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.out_file, &key.to_vec())?;
    Msg::ok(format!(
        "New Keypair created and written to {:?}",
        args.out_file.display()
    ))
}

fn get_keypair(path: Option<PathBuf>) -> (String, String) {
    match path {
        None => ("unset".to_string(), "unset".to_string()),
        Some(path) => {
            let display_path = path.as_path().display().to_string();
            match fs::read(path).with_context(|| format!("path does not exist: {display_path}")) {
                Err(e) => (e.to_string(), "".to_string()),
                Ok(data) => match Keypair::try_from(&data[..]) {
                    Err(e) => (display_path, e.to_string()),
                    Ok(keypair) => (display_path, keypair.public_key().to_string()),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::{env, fs};
    use temp_dir::TempDir;

    use crate::{
        cmds::{
            self,
            env::{env_info, generate_keypair, get_keypair},
            EnvInfo, GenerateKeypair,
        },
        hex_field,
    };

    #[test]
    fn env_info_test() {
        // Make the keypairs to be referenced
        let dir = TempDir::new().unwrap();
        let env_keypair = dir.child("env-keypair.bin");
        let arg_keypair = dir.child("arg-keypair.bin");
        generate_keypair(GenerateKeypair {
            out_file: env_keypair.clone(),
            commit: true,
        })
        .unwrap();
        generate_keypair(GenerateKeypair {
            out_file: arg_keypair.clone(),
            commit: true,
        })
        .unwrap();

        // Set the environment and arguments
        env::set_var(cmds::ENV_CONFIG_HOST, "env-localhost:1337");
        env::set_var(cmds::ENV_NET_ID, "C0053");
        env::set_var(cmds::ENV_OUI, "42");
        env::set_var(cmds::ENV_MAX_COPIES, "42");
        env::set_var(cmds::ENV_KEYPAIR_BIN, env_keypair.clone());

        let env_args = EnvInfo {
            config_host: Some("arg-localhost:1337".to_string()),
            keypair: Some(arg_keypair.clone()),
            net_id: Some(hex_field::net_id(42)),
            oui: Some(4),
            max_copies: Some(1337),
        };

        // =======
        let output = env_info(env_args).unwrap().into_inner();
        let s: serde_json::Value = serde_json::from_str(&output).unwrap();

        let env = &s["environment"];
        let arg = &s["arguments"];

        let string_not_empty =
            |val: &serde_json::Value| !val.as_str().unwrap().to_string().is_empty();

        assert_eq!(env[cmds::ENV_CONFIG_HOST], "env-localhost:1337");
        assert_eq!(env[cmds::ENV_NET_ID], "C0053");
        assert_eq!(env[cmds::ENV_OUI], "42");
        assert_eq!(env[cmds::ENV_MAX_COPIES], "42");
        assert_eq!(
            env[cmds::ENV_KEYPAIR_BIN],
            env_keypair.display().to_string()
        );
        assert!(string_not_empty(&env["public_key_from_keypair"]));

        assert_eq!(arg["config_host"], "arg-localhost:1337");
        assert_eq!(arg["keypair"], arg_keypair.display().to_string());
        assert!(string_not_empty(&arg["public_key_from_keypair"]));
        assert_eq!(arg["net_id"], "00002A");
        assert_eq!(arg["oui"], 4);
        assert_eq!(arg["max_copies"], 1337);
    }

    #[test]
    fn get_keypair_does_not_exist() {
        let (location, pubkey) = get_keypair(Some("./nowhere.bin".into()));
        assert_eq!(location, "path does not exist: ./nowhere.bin");
        assert!(pubkey.is_empty());
    }

    #[test]
    fn get_keypair_invalid() {
        // Write an invalid keypair
        let dir = TempDir::new().unwrap();
        let arg_keypair = dir.child("arg-keypair.bin");
        fs::write(arg_keypair.clone(), "invalid key").unwrap();

        // =======
        let (location, pubkey) = get_keypair(Some(arg_keypair.clone()));
        assert_eq!(location, arg_keypair.display().to_string());
        assert_eq!(pubkey, "decode error");
    }

    #[test]
    fn get_keypair_not_provided() {
        let (location, pubkey) = get_keypair(None);
        assert_eq!(location, "unset");
        assert_eq!(pubkey, "unset");
    }
}
