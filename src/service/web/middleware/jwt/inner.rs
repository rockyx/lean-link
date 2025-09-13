use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use rand::distr::SampleString;

#[derive(Clone)]
pub(crate) struct Inner {
    pub secret_key: String,
    pub algorithm: Algorithm,
}

impl Inner {
    pub fn new(secret_key: String, algorithm: Algorithm) -> Self {
        Self {
            secret_key,
            algorithm,
        }
    }

    pub fn validate(&self, token: &str) -> Result<super::Claims, crate::errors::Error> {
        let validation = Validation::new(self.algorithm);

        let token_data = decode::<super::Claims>(
            token,
            &DecodingKey::from_secret(self.secret_key.as_bytes()),
            &validation,
        )?;

        let claims = token_data.claims;

        Ok(claims)
    }
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            secret_key: rand::distr::Alphanumeric::default().sample_string(&mut rand::rng(), 32),
            algorithm: Algorithm::HS256,
        }
    }
}
