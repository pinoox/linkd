use linkd_core::{HumanError, LinkdError, LinkdResult};

pub fn print_error(err: &LinkdError) {
    let human = HumanError::from_error(err);
    eprint!("{}", human.display());
}

pub fn print_result<T>(result: LinkdResult<T>) -> anyhow::Result<T> {
    match result {
        Ok(v) => Ok(v),
        Err(e) => {
            print_error(&e);
            Err(anyhow::anyhow!(e.to_string()))
        }
    }
}
