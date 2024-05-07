pub struct Constants {
    pub default_webhook_id: u64,
    pub default_webhook_token: String,
    pub webhook_closed: String,
}

impl Constants {
    fn new() -> Self {
        Constants {
            default_webhook_id: get_dotenv_id::<u64>("WEBHOOK_ID"),
            default_webhook_token: get_dotenv("WEBHOOK_TOKEN"),
            webhook_closed: "Channels got closed unexpectedly!".to_string(),
        }
    }
    #[allow(static_mut_refs)]
    pub fn get_constants() -> &'static Self {
        //This should be fine, since this static is not mutable outside this function
        tracing::debug!("Getting Constants");
        static mut CONSTANTS: Option<Constants> = None;
        unsafe { &mut CONSTANTS }.get_or_insert_with(|| {
            tracing::info!("Generating Constants");
            Constants::new()
        })
    }
}

fn get_dotenv(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("There is some error with the setting {} in dotenv!", name))
}

fn get_dotenv_id<T: std::str::FromStr>(name: &str) -> T {
    let type_name = std::any::type_name::<T>();
    get_dotenv(name).parse::<T>().unwrap_or_else(|_| {
        panic!(
            "The String was correctly read, but could not be parsed into {}",
            type_name,
        )
    })
}
