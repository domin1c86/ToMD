use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use thiserror::Error;
use uuid::Uuid;

use crate::ProviderConfig;

pub const SERVICE_NAME: &str = "com.tomd.designlanguageextractor";

pub trait CredentialStore: Send + Sync {
    fn create_secret(
        &self,
        service: &str,
        username: &str,
        secret: &str,
    ) -> Result<(), CredentialStoreError>;

    fn replace_secret(
        &self,
        service: &str,
        username: &str,
        secret: &str,
    ) -> Result<(), CredentialStoreError>;

    fn read_secret(
        &self,
        service: &str,
        username: &str,
    ) -> Result<Option<String>, CredentialStoreError>;

    fn delete_secret(&self, service: &str, username: &str) -> Result<(), CredentialStoreError>;
}

#[derive(Debug, Error)]
pub enum CredentialStoreError {
    #[error("credential reference is invalid: {0}")]
    InvalidCredentialRef(String),
    #[error("credential store lock was poisoned")]
    LockPoisoned,
    #[error("operating-system credential store failed")]
    Keyring(#[from] keyring::Error),
}

#[derive(Debug, Clone, Default)]
pub struct MemoryCredentialStore {
    secrets: Arc<Mutex<HashMap<(String, String), String>>>,
}

impl CredentialStore for MemoryCredentialStore {
    fn create_secret(
        &self,
        service: &str,
        username: &str,
        secret: &str,
    ) -> Result<(), CredentialStoreError> {
        self.set_secret(service, username, secret)
    }

    fn replace_secret(
        &self,
        service: &str,
        username: &str,
        secret: &str,
    ) -> Result<(), CredentialStoreError> {
        self.set_secret(service, username, secret)
    }

    fn read_secret(
        &self,
        service: &str,
        username: &str,
    ) -> Result<Option<String>, CredentialStoreError> {
        Ok(self
            .secrets
            .lock()
            .map_err(|_| CredentialStoreError::LockPoisoned)?
            .get(&(service.to_owned(), username.to_owned()))
            .cloned())
    }

    fn delete_secret(&self, service: &str, username: &str) -> Result<(), CredentialStoreError> {
        self.secrets
            .lock()
            .map_err(|_| CredentialStoreError::LockPoisoned)?
            .remove(&(service.to_owned(), username.to_owned()));
        Ok(())
    }
}

impl MemoryCredentialStore {
    fn set_secret(
        &self,
        service: &str,
        username: &str,
        secret: &str,
    ) -> Result<(), CredentialStoreError> {
        self.secrets
            .lock()
            .map_err(|_| CredentialStoreError::LockPoisoned)?
            .insert((service.to_owned(), username.to_owned()), secret.to_owned());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyringCredentialStore;

impl CredentialStore for KeyringCredentialStore {
    fn create_secret(
        &self,
        service: &str,
        username: &str,
        secret: &str,
    ) -> Result<(), CredentialStoreError> {
        keyring::Entry::new(service, username)?.set_password(secret)?;
        Ok(())
    }

    fn replace_secret(
        &self,
        service: &str,
        username: &str,
        secret: &str,
    ) -> Result<(), CredentialStoreError> {
        keyring::Entry::new(service, username)?.set_password(secret)?;
        Ok(())
    }

    fn read_secret(
        &self,
        service: &str,
        username: &str,
    ) -> Result<Option<String>, CredentialStoreError> {
        match keyring::Entry::new(service, username)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn delete_secret(&self, service: &str, username: &str) -> Result<(), CredentialStoreError> {
        match keyring::Entry::new(service, username)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

pub fn credential_ref_for_provider(provider_id: Uuid) -> String {
    format!(
        "keyring://{SERVICE_NAME}/{}",
        username_for_provider(provider_id)
    )
}

pub fn save_provider_with_store(
    store: &impl CredentialStore,
    mut config: ProviderConfig,
    secret: &str,
) -> Result<ProviderConfig, CredentialStoreError> {
    config.credential_ref = credential_ref_for_provider(config.id);
    let (service, username) = credential_location(&config)?;
    store.create_secret(service, username, secret)?;
    Ok(config)
}

pub fn replace_provider_secret_with_store(
    store: &impl CredentialStore,
    config: &ProviderConfig,
    secret: &str,
) -> Result<(), CredentialStoreError> {
    let (service, username) = credential_location(config)?;
    store.replace_secret(service, username, secret)
}

pub fn read_provider_secret_with_store(
    store: &impl CredentialStore,
    config: &ProviderConfig,
) -> Result<Option<String>, CredentialStoreError> {
    let (service, username) = credential_location(config)?;
    store.read_secret(service, username)
}

pub fn delete_provider_secret_with_store(
    store: &impl CredentialStore,
    config: &ProviderConfig,
) -> Result<(), CredentialStoreError> {
    let (service, username) = credential_location(config)?;
    store.delete_secret(service, username)
}

fn credential_location(config: &ProviderConfig) -> Result<(&str, &str), CredentialStoreError> {
    let prefix = format!("keyring://{SERVICE_NAME}/");
    let username = config
        .credential_ref
        .strip_prefix(&prefix)
        .ok_or_else(|| CredentialStoreError::InvalidCredentialRef(config.credential_ref.clone()))?;

    if username == username_for_provider(config.id) {
        Ok((SERVICE_NAME, username))
    } else {
        Err(CredentialStoreError::InvalidCredentialRef(
            config.credential_ref.clone(),
        ))
    }
}

fn username_for_provider(provider_id: Uuid) -> String {
    format!("provider:{provider_id}")
}
