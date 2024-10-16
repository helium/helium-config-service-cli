use anyhow::Result;
use helium_crypto::{Keypair, PublicKey};
use helium_proto::Message;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64)
}

pub trait MsgSign: Message + std::clone::Clone {
    fn sign(&self, keypair: &Keypair) -> Result<Vec<u8>>
    where
        Self: std::marker::Sized;
}

#[macro_export]
macro_rules! impl_sign {
    ($msg_type:ty, $( $sig: ident ),+ ) => {
        impl crate::clients::utils::MsgSign for $msg_type {
            fn sign(&self, keypair: &Keypair) -> Result<Vec<u8>> {
                let mut msg = self.clone();
                $(msg.$sig = vec![];)+
                Ok(helium_crypto::Sign::sign(keypair, &msg.encode_to_vec())?)
            }
        }
    }
}

pub trait MsgVerify: Message + std::clone::Clone {
    fn verify(&self, verifier: &PublicKey) -> Result<()>
    where
        Self: std::marker::Sized;
}

#[macro_export]
macro_rules! impl_verify {
    ($msg_type:ty, $sig: ident) => {
        impl crate::clients::utils::MsgVerify for $msg_type {
            fn verify(&self, verifier: &PublicKey) -> Result<()> {
                let mut buf = vec![];
                let mut msg = self.clone();
                msg.$sig = vec![];
                msg.encode(&mut buf)?;
                helium_crypto::Verify::verify(verifier, &buf, &self.$sig)
                    .map_err(anyhow::Error::from)
            }
        }
    };
}
