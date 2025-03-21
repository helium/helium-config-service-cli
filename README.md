# Helium Config Service CLI

Cli tool to interact with [Helium Config Service](https://github.com/helium/oracles/tree/main/iot_config).

## Installation

### From Binary

Download the latest binary for your platform here from
[Releases](https://github.com/helium/helium-config-service-cli/releases/latest). Unpack
the zip file and place the `helium-config-service-cli` binary in your `$PATH`
somewhere.

## Usage

At any time use `-h` or `--help` to get more help for a command.

### Create a keypair

If you dont already have a helium keypair for interacting with this cli then lets generate one

```
    helium-config-service-cli env generate-keypair
```

### Init the env

```
    helium-config-service-cli env init
        ----- Leave blank to ignore...
    Config Service Host: http://0.0.0.0:8000
    Solana RPC URL: http://127.0.0.1:8999
    Keypair Location: ./keypair.bin
    ----- Enter all zeros to ignore...
    Net ID: 000000
    ----- Enter zero to ignore...
    Assigned OUI: 0
    Default Max Copies: 0

    Put these in your environment
    ------------------------------------
    HELIUM_CONFIG_HOST=http://0.0.0.0:8080
    SOLANA_URL=http://127.0.0.1:8899
    HELIUM_KEYPAIR_BIN=./keypair.bin
```

### Displaying

Lets display the information about your env

```
    HELIUM_CONFIG_HOST=<config_host> HELIUM_KEYPAIR_BIN=<./keypairn.bin> SOLANA_URL=<solana_url> helium-config-service-cli env info
    {
      "arguments": {
        "config_host": "http://0.0.0.0:8080",
        "helium_public_key_from_keypair": "14VeYhArVg1P6jN5pX4Czq61gjtAHZ2fz6Gh14ms1F4TNf1NQro",
        "key_type_from_keypair": "ed25519",
        "keypair": "./keypair2.bin",
        "max_copies": null,
        "net_id": null,
        "oui": null,
        "solana_public_key_from_keypair": "EjrsaU42xeuMqACKuseU3FFHRZyCmfeFATNmMAae9HTp",
        "solana_url": "http://127.0.0.1:8899"
      },
      "environment": {
        "HELIUM_CONFIG_HOST": "http://0.0.0.0:8080",
        "HELIUM_KEYPAIR_BIN": "./keypair2.bin",
        "HELIUM_MAX_COPIES": "unset",
        "HELIUM_NET_ID": "unset",
        "HELIUM_OUI": "unset",
        "SOLANA_URL": "http://127.0.0.1:8899",
        "helium_public_key_from_keypair": "14VeYhArVg1P6jN5pX4Czq61gjtAHZ2fz6Gh14ms1F4TNf1NQro",
        "key_type_from_keypair": "ed25519",
        "solana_public_key_from_keypair": "EjrsaU42xeuMqACKuseU3FFHRZyCmfeFATNmMAae9HTp"
      }
    }
```

Here you'll notice all the info about your env and your keypair.

### Org commands

Going forward im ommiting the env vars from the commands for simplicity sake. If you want to include all the requried ones they should be `HELIUM_CONFIG_HOST`, `HELIUM_KEYPAIR_BIN`, `HELIUM_CONFIG_PUBKEY`, `SOLANA_URL`.

Commands that interact direclty with solana require the account to have [SOL](https://docs.helium.com/tokens/sol-token) for executing the transactions. Certain commands also require [DC](https://docs.helium.com/tokens/data-credit) to be present in the account.

```
    helium-config-service-cli org create-helium --owner <helium publickey> --net-id <net id type> --commit
    == Helium Organization Created: 2242 ==
    == Call `org get --oui 2242 to see its details` ==
```

```
    helium-config-service-cli org approve --oui 2242 --commit
    == Organization Approved: 2242 ==
```

```
    helium-config-service-cli org enable --oui 2242 --commit
    OUI 2242 enabled
```

```
    helium-config-service-cli org update devaddr-constraint-add --oui 2245 --num-blocks 3 --commit
    == Organization Updated ==
    == Call `org get --oui 2242 to see its details ==

    helium-config-service-cli org get --oui 2242
    {
      "org": {
        "oui": 2242,
        "address": <helium publickey>,
        "owner": <helium publickey>,
        "escrow_key": "OUI_2242",
        "delegate_keys": [
          <helium publickey>
        ],
        "approved": true,
        "locked": false
      },
      "net_id": "00003C",
      "devaddr_constraints": [
        {
          "start_addr": "780001B9",
          "end_addr": "780001D1"
        }
      ]
    }
```
