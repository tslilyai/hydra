use mysql::*;

pub mod crypto;
pub mod spec;

pub struct HydraClient {
    pub pool: mysql::Pool,
}

impl HydraClient {
    pub fn new(
        user: &str,
        password: &str,
        host: &str,
        dbname: &str,
        in_memory: bool,
    ) -> HydraClient {
        let url = format!("mysql://{}:{}@{}/{}", user, password, host, dbname);
        let pool = mysql::Pool::new(Opts::from_url(&url).unwrap()).unwrap();
        HydraClient { pool: pool.clone() }
    }

    pub fn create_fake_user() {
        //
    }
}
