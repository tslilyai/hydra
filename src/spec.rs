use fast_paths::*;
use log::info;
use mysql::prelude::*;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;

pub type TableName = String;
pub type ColName = String;

pub struct Spec {
    tables: Vec<TableName>,
    tab2ix: HashMap<TableName, usize>,
    link2fks: HashMap<(TableName, TableName), Link>,
    path_calculator: PathCalculator,
    fast_graph: FastGraph,

    // how to create fake users (which tables to insert which values)
    user_spec: Vec<TableSpec>,
}

impl Spec {
    pub fn new(tables: &Vec<TableName>, links: &Vec<Link>) -> Spec {
        let mut tab2ix: HashMap<TableName, usize> = HashMap::new();
        let mut link2fks: HashMap<(TableName, TableName), Link> = HashMap::new();

        tables.iter().enumerate().for_each(|(i, t)| {
            tab2ix.insert(t.clone(), i);
        });

        let mut input_graph = InputGraph::new();
        for link in links {
            link2fks.insert((link.src.clone(), link.dest.clone()), link.clone());
            let srcix = tab2ix.get(&link.src).unwrap();
            let destix = tab2ix.get(&link.dest).unwrap();
            input_graph.add_edge(*srcix, *destix, 1);
        }
        input_graph.freeze();
        let fast_graph = fast_paths::prepare(&input_graph);
        let mut path_calc = fast_paths::create_calculator(&fast_graph);

        Spec {
            tables: tables.clone(),
            user_spec: vec![],
            tab2ix: tab2ix,
            link2fks: link2fks,
            path_calculator: path_calc,
            fast_graph: fast_graph,
        }
    }

    pub fn create_user<Q: Queryable>(&self, db: &mut Q) -> mysql::Result<()> {
        for ts in &self.user_spec {
            ts.insert_row(db)?;
        }
        Ok(())
    }

    // TODO DFS through graph to get connection from table to target
    pub fn query_with_filters(&mut self, target: &TableName, filters: &Vec<Filter>) -> String {
        let mut joinstr: Vec<String> = vec![];
        let mut filterstr: Vec<String> = vec![];
        let mut joined = HashSet::new();
        joined.insert(target.clone());
        for f in filters {
            filterstr.push(format!("{}.{} = {}", f.table, f.col, f.val.as_sql(false)));
            if &f.table != target {
                // don't look for path if we've already included it in the join
                if joined.get(&f.table).is_none() {
                    let srcix = self.tab2ix.get(&f.table).unwrap();
                    let destix = self.tab2ix.get(target).unwrap();
                    let path = self
                        .path_calculator
                        .calc_path(&self.fast_graph, *srcix, *destix);
                    match path {
                        Some(p) => {
                            let nodes = p.get_nodes();
                            for (i, n) in nodes.iter().enumerate() {
                                // stop at the target
                                if i < nodes.len() - 1 {
                                    let src = self.tables[*n].clone();
                                    let dest = self.tables[nodes[i + 1]].clone();
                                    let link = self.link2fks.get(&(src.clone(), dest)).unwrap();
                                    if joined.get(&src).is_none() {
                                        joinstr.push(format!(
                                            "{} ON {}.{} = {}.{}",
                                            link.src,
                                            link.src,
                                            link.src_fk,
                                            link.dest,
                                            link.dest_fk,
                                        ));
                                        joined.insert(src);
                                    }
                                }
                            }
                        }
                        None => unimplemented!("No path to table {} from {}?", target, f.table),
                    }
                }
            }
        }

        let q = format!(
            "SELECT * FROM {} {} {}",
            target,
            joinstr.join(" JOIN "),
            filterstr.join(" AND ")
        );
        info!("query with filters: {}", q);
        q
    }
}

// INSERT INTO _ (col1, col2, ..) VALUES (v1,_,_)
pub struct TableSpec {
    table: TableName,
    columns: Vec<ColName>,
    values: Vec<ValueSpec>,
}

impl TableSpec {
    pub fn insert_row<Q: Queryable>(&self, db: &mut Q) -> mysql::Result<()> {
        let values: Vec<String> = self
            .values
            .iter()
            .map(|v| valuespec2value(v).as_sql(false))
            .collect();
        let q = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            self.columns.join(","),
            values.join(",")
        );
        db.query_drop(q)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Link {
    src: TableName,
    dest: TableName,
    src_fk: ColName,
    dest_fk: ColName,
}

// WHERE tab.col = val
pub struct Filter {
    table: TableName,
    col: ColName,
    val: mysql::Value,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ValueSpec {
    ConstNum(u64),
    ConstStr(String),
    RandNum { lb: usize, ub: usize },
    RandStr { len: usize },
    RandEmail,
    RandPhone,
    ConstDate { year: u16, month: u8, day: u8 },
    Bool(bool),
    Null,
}

pub fn valuespec2value(vs: &ValueSpec) -> mysql::Value {
    use mysql::Value::*;
    use ValueSpec::*;
    match vs {
        ConstNum(n) => UInt(*n),
        ConstStr(s) => Bytes(s.clone().into_bytes()),
        RandNum { lb, ub } => {
            let mut rng = rand::thread_rng();
            UInt(rng.gen_range(*lb..*ub).try_into().unwrap())
        }
        RandStr { len } => {
            let rng = rand::thread_rng();
            let rand_string: String = rng
                .sample_iter(&Alphanumeric)
                .take(*len)
                .map(char::from)
                .collect();
            Bytes(rand_string.into_bytes())
        }
        RandEmail => {
            let rng = rand::thread_rng();
            let rand_string: String = rng
                .sample_iter(&Alphanumeric)
                .take(20)
                .map(char::from)
                .collect();
            Bytes(format!("{}@anon.com", rand_string).into_bytes())
        }
        RandPhone => {
            let mut rng = rand::thread_rng();
            const CHARSET: &[u8] = b"0123456789";
            const LEN: usize = 9;
            let rand_phone: String = (0..LEN)
                .map(|_| {
                    let idx = rng.gen_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect();
            Bytes(rand_phone.into_bytes())
        }
        Bool(b) => match b {
            true => Int(1),
            false => Int(0),
        },
        ConstDate { year, month, day } => Date(*year, *month, *day, 0, 0, 0, 0),
        Null => NULL,
    }
}
