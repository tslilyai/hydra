use mysql::*;

pub mod crypto;
pub mod spec;

use spec::*;

pub struct HydraClient {
    pub pool: mysql::Pool,
    pub spec: Spec,
}

impl HydraClient {
    pub fn new(
        user: &str,
        password: &str,
        host: &str,
        dbname: &str,
        in_memory: bool,
        spec: Spec,
    ) -> HydraClient {
        let url = format!("mysql://{}:{}@{}/{}", user, password, host, dbname);
        let pool = mysql::Pool::new(Opts::from_url(&url).unwrap()).unwrap();
        HydraClient {
            pool: pool.clone(),
            spec: Spec::new(
                &vec![],
                &vec![],
                UserSpec {
                    tables: vec![],
                    id: (String::new(), String::new()),
                },
            ),
        }
    }

    pub fn create_fake_user(&mut self) -> Result<UID> {
        let mut db = self.pool.get_conn()?;
        self.spec.create_user(&mut db)
    }

    pub fn reassign_data(
        &mut self,
        src_usr: &UID,
        dest_usr: &UID,
        datatable: &TableName,
        filters: &Vec<Filter>,
    ) {
    }
}
