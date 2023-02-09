use mysql::*;

pub mod crypto;
pub mod spec;

pub struct HydraClient {
    pub pool: mysql::Pool,
    pub spec: spec::Spec,
}

impl HydraClient {
    pub fn new(
        user: &str,
        password: &str,
        host: &str,
        dbname: &str,
        in_memory: bool,
        spec: spec::Spec,
    ) -> HydraClient {
        let url = format!("mysql://{}:{}@{}/{}", user, password, host, dbname);
        let pool = mysql::Pool::new(Opts::from_url(&url).unwrap()).unwrap();
        HydraClient {
            pool: pool.clone(),
            spec: spec::Spec::new(),
        }
    }

    pub fn create_fake_user(&mut self) -> Result<()> {
        let mut db = self.pool.get_conn()?;
        self.spec.create_user(&mut db)
    }

    pub fn reassign_data(data_id: DataID, src_usr: UID, dest_usr: UID) {
        //
    }
}
