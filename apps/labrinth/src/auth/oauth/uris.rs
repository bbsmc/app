use super::errors::OAuthError;
use crate::auth::oauth::OAuthErrorType;
use crate::database::models::OAuthClientId;
use serde::{Deserialize, Serialize};

#[derive(derive_new::new, Serialize, Deserialize)]
pub struct OAuthRedirectUris {
    pub original: Option<String>,
    pub validated: ValidatedRedirectUri,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedRedirectUri(pub String);

impl ValidatedRedirectUri {
    pub fn validate<'a>(
        to_validate: &Option<String>,
        validate_against: impl IntoIterator<Item = &'a str> + Clone,
        client_id: OAuthClientId,
    ) -> Result<Self, OAuthError> {
        if let Some(first_client_redirect_uri) =
            validate_against.clone().into_iter().next()
        {
            if let Some(to_validate) = to_validate {
                if validate_against.into_iter().any(|uri| {
                    same_uri_except_query_components(uri, to_validate)
                }) {
                    Ok(ValidatedRedirectUri(to_validate.clone()))
                } else {
                    Err(OAuthError::error(
                        OAuthErrorType::RedirectUriNotConfigured(
                            to_validate.clone(),
                        ),
                    ))
                }
            } else {
                Ok(ValidatedRedirectUri(first_client_redirect_uri.to_string()))
            }
        } else {
            Err(OAuthError::error(
                OAuthErrorType::ClientMissingRedirectURI { client_id },
            ))
        }
    }
}

fn same_uri_except_query_components(a: &str, b: &str) -> bool {
    let mut a_components = a.split('?');
    let mut b_components = b.split('?');
    a_components.next() == b_components.next()
}
