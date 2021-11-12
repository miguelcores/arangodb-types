use arangors::ClientError;

pub fn check_client_is_write_conflict(error: ClientError) -> Result<ClientError, ClientError> {
    match &error {
        ClientError::Arango(e) => match e.error_num() {
            1200 => Ok(error),
            _ => Err(error),
        },
        _ => Err(error),
    }
}
