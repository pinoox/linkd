use std::fs;

use linkd_core::{
    auth_token_path, ensure_home, set_owner_only_permissions, LinkdError, LinkdResult,
};

pub fn load_auth_token() -> LinkdResult<String> {
    ensure_home().map_err(|e| LinkdError::io(linkd_core::linkd_home(), e))?;
    let path = auth_token_path();
    if path.exists() {
        let token = fs::read_to_string(&path).map_err(|e| LinkdError::io(&path, e))?;
        return Ok(token.trim().to_string());
    }
    ensure_auth_token()
}

pub fn ensure_auth_token() -> LinkdResult<String> {
    ensure_home().map_err(|e| LinkdError::io(linkd_core::linkd_home(), e))?;
    let path = auth_token_path();
    if path.exists() {
        return load_auth_token();
    }

    let token = uuid::Uuid::new_v4().to_string();
    fs::write(&path, &token).map_err(|e| LinkdError::io(&path, e))?;
    let _ = set_owner_only_permissions(&path);
    Ok(token)
}

pub fn verify_auth_token(provided: &str) -> LinkdResult<()> {
    let expected = load_auth_token()?;
    if provided == expected {
        Ok(())
    } else {
        Err(LinkdError::InvalidAuthToken)
    }
}
