use auth::*;
use mysql::*;
use spec::*;

pub mod auth;
pub mod crypto;
pub mod spec;

pub struct Hydra {
    pub pool: mysql::Pool,
    pub spec: Spec,
    pub authorizer: Authorizer,
}

impl Hydra {
    pub fn new(user: &str, password: &str, host: &str, dbname: &str, in_memory: bool) -> Hydra {
        let url = format!("mysql://{}:{}@{}/{}", user, password, host, dbname);
        let pool = mysql::Pool::new(Opts::from_url(&url).unwrap()).unwrap();

        Hydra {
            pool: pool.clone(),
            authorizer: Authorizer::new(),
            spec: Spec::new(
                &vec![],
                &vec![],
                ObjectSpec {
                    tables: vec![],
                    id: (String::new(), String::new()),
                },
            ),
        }
    }

    pub fn register_user(&mut self, uid: &UID, pass: &str) -> Result<()> {
        let user_share = self.authorizer.register_user_shares(uid, pass);
        Ok(())
    }

    pub fn create_fake_user(&mut self) -> Result<UID> {
        // don't need to register key
        let mut db = self.pool.get_conn()?;
        self.spec.create_user(&mut db)
    }

    pub fn connect_user_to(&mut self, uid: &UID, to: &UID) -> Result<()> {
        Ok(())
    }

    pub fn reassign_data(
        &mut self,
        src_usr: &UID,
        dest_usr: &UID,
        datatable: &TableName,
        filters: &Vec<Filter>,
    ) {
    }

    pub fn delete_user(&mut self, uid: &UID) {}
}
