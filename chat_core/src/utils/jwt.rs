use std::collections::HashSet;

use jwt_simple::{
    claims::Claims,
    common::VerificationOptions,
    prelude::{Duration, Ed25519KeyPair, Ed25519PublicKey, EdDSAKeyPairLike, EdDSAPublicKeyLike},
};

use crate::User;

const JWT_DURATION: u64 = 60 * 60 * 24 * 7;
const JWT_ISSUER: &str = "chat_server";
const JWT_AUD: &str = "chat_web";

pub struct EncodingKey(Ed25519KeyPair);

#[allow(unused)]
pub struct DecodingKey(Ed25519PublicKey);

impl EncodingKey {
    pub fn load(pem: &str) -> Result<Self, jwt_simple::Error> {
        Ok(Self(Ed25519KeyPair::from_pem(pem)?))
    }

    pub fn sign(&self, user: impl Into<User>) -> Result<String, jwt_simple::Error> {
        let claims = Claims::with_custom_claims(user.into(), Duration::from_secs(JWT_DURATION));
        let claims = claims.with_issuer(JWT_ISSUER).with_audience(JWT_AUD);
        self.0.sign(claims)
    }
}

impl DecodingKey {
    pub fn load(pem: &str) -> Result<Self, jwt_simple::Error> {
        Ok(Self(Ed25519PublicKey::from_pem(pem)?))
    }

    #[allow(unused)]
    pub fn verify(&self, token: &str) -> Result<User, jwt_simple::Error> {
        let opts = VerificationOptions {
            allowed_issuers: Some(HashSet::from([JWT_ISSUER.to_string()])),
            allowed_audiences: Some(HashSet::from([JWT_AUD.to_string()])),
            ..Default::default()
        };
        // opts.allowed_issuers = Some(HashSet::from([JWT_ISSUER.to_string()]));
        // opts.allowed_audiences = Some(HashSet::from([JWT_AUD.to_string()]));
        let claims = self.0.verify_token::<User>(token, Some(opts))?;
        Ok(claims.custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn jwt_sign_verify_should_work() -> Result<()> {
        let ek_str = include_str!("../../fixtures/encoding.pem");
        let dk_str = include_str!("../../fixtures/decoding.pem");
        let ek = EncodingKey::load(ek_str)?;
        let dk = DecodingKey::load(dk_str)?;

        let user = User::new(1, 0, "wiki", "charmfocus@gmail.com");

        let token = ek.sign(user.clone())?;
        let user2 = dk.verify(&token)?;
        assert_eq!(user, user2);
        Ok(())
    }
}
