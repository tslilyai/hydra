use log::{debug, warn};
use mysql::prelude::*;
use mysql::Opts;
use std::io;
use std::str::FromStr;

pub const NULLSTR: &'static str = "NULL";

pub fn get_msql_srv_coltype(t: &mysql::consts::ColumnType) -> msql_srv::ColumnType {
    use msql_srv::ColumnType;
    match t {
        mysql::consts::ColumnType::MYSQL_TYPE_DECIMAL => ColumnType::MYSQL_TYPE_DECIMAL,
        mysql::consts::ColumnType::MYSQL_TYPE_TINY => ColumnType::MYSQL_TYPE_TINY,
        mysql::consts::ColumnType::MYSQL_TYPE_SHORT => ColumnType::MYSQL_TYPE_SHORT,
        mysql::consts::ColumnType::MYSQL_TYPE_LONG => ColumnType::MYSQL_TYPE_LONG,
        mysql::consts::ColumnType::MYSQL_TYPE_FLOAT => ColumnType::MYSQL_TYPE_FLOAT,
        mysql::consts::ColumnType::MYSQL_TYPE_DOUBLE => ColumnType::MYSQL_TYPE_DOUBLE,
        mysql::consts::ColumnType::MYSQL_TYPE_NULL => ColumnType::MYSQL_TYPE_NULL,
        mysql::consts::ColumnType::MYSQL_TYPE_TIMESTAMP => ColumnType::MYSQL_TYPE_TIMESTAMP,
        mysql::consts::ColumnType::MYSQL_TYPE_LONGLONG => ColumnType::MYSQL_TYPE_LONGLONG,
        mysql::consts::ColumnType::MYSQL_TYPE_INT24 => ColumnType::MYSQL_TYPE_INT24,
        mysql::consts::ColumnType::MYSQL_TYPE_DATE => ColumnType::MYSQL_TYPE_DATE,
        mysql::consts::ColumnType::MYSQL_TYPE_TIME => ColumnType::MYSQL_TYPE_TIME,
        mysql::consts::ColumnType::MYSQL_TYPE_DATETIME => ColumnType::MYSQL_TYPE_DATETIME,
        mysql::consts::ColumnType::MYSQL_TYPE_YEAR => ColumnType::MYSQL_TYPE_YEAR,
        mysql::consts::ColumnType::MYSQL_TYPE_NEWDATE => ColumnType::MYSQL_TYPE_NEWDATE,
        mysql::consts::ColumnType::MYSQL_TYPE_VARCHAR => ColumnType::MYSQL_TYPE_VARCHAR,
        mysql::consts::ColumnType::MYSQL_TYPE_BIT => ColumnType::MYSQL_TYPE_BIT,
        mysql::consts::ColumnType::MYSQL_TYPE_TIMESTAMP2 => ColumnType::MYSQL_TYPE_TIMESTAMP2,
        mysql::consts::ColumnType::MYSQL_TYPE_DATETIME2 => ColumnType::MYSQL_TYPE_DATETIME2,
        mysql::consts::ColumnType::MYSQL_TYPE_TIME2 => ColumnType::MYSQL_TYPE_TIME2,
        mysql::consts::ColumnType::MYSQL_TYPE_JSON => ColumnType::MYSQL_TYPE_JSON,
        mysql::consts::ColumnType::MYSQL_TYPE_NEWDECIMAL => ColumnType::MYSQL_TYPE_NEWDECIMAL,
        mysql::consts::ColumnType::MYSQL_TYPE_ENUM => ColumnType::MYSQL_TYPE_ENUM,
        mysql::consts::ColumnType::MYSQL_TYPE_SET => ColumnType::MYSQL_TYPE_SET,
        mysql::consts::ColumnType::MYSQL_TYPE_TINY_BLOB => ColumnType::MYSQL_TYPE_TINY_BLOB,
        mysql::consts::ColumnType::MYSQL_TYPE_MEDIUM_BLOB => ColumnType::MYSQL_TYPE_MEDIUM_BLOB,
        mysql::consts::ColumnType::MYSQL_TYPE_LONG_BLOB => ColumnType::MYSQL_TYPE_LONG_BLOB,
        mysql::consts::ColumnType::MYSQL_TYPE_BLOB => ColumnType::MYSQL_TYPE_BLOB,
        mysql::consts::ColumnType::MYSQL_TYPE_VAR_STRING => ColumnType::MYSQL_TYPE_VAR_STRING,
        mysql::consts::ColumnType::MYSQL_TYPE_STRING => ColumnType::MYSQL_TYPE_STRING,
        mysql::consts::ColumnType::MYSQL_TYPE_GEOMETRY => ColumnType::MYSQL_TYPE_GEOMETRY,
        _ => unimplemented!("Unsupported coltype in msql_srv"),
        //mysql::consts::ColumnType::MYSQL_TYPE_TYPED_ARRAY => ColumnType::MYSQL_TYPE_TYPED_ARRAY,
        //mysql::consts::ColumnType::MYSQL_TYPE_UNKNOWN => ColumnType::MYSQL_TYPE_UNKNOWN,
    }
}

/*****************************************
 * SELECT
 ****************************************/
fn is_string_numeric(str: &str) -> bool {
    for c in str.chars() {
        if !c.is_numeric() {
            return false;
        }
    }
    return true;
}

pub fn str_select_statement(table: &str, from: &str, selection: &str) -> String {
    let s = if from == "" {
        format!("SELECT {}.* FROM {} WHERE {}", table, table, selection)
    } else {
        format!("SELECT {}.* FROM {} WHERE {}", table, from, selection)
    };
    debug!("{}", s);
    s
}

/************************************
 * INITIALIZATION HELPERS
 * **********************************/
fn create_schema(db: &mut mysql::Conn, in_memory: bool, schema: &str) -> mysql::Result<()> {
    db.query_drop("SET max_heap_table_size = 4294967295;")?;

    /* issue schema statements */
    let mut sql = String::new();
    let mut stmt = String::new();
    for line in schema.lines() {
        warn!("Got line {}", line);
        if line.starts_with("--") || line.is_empty() {
            continue;
        }
        if !sql.is_empty() {
            sql.push_str(" ");
            stmt.push_str(" ");
        }
        stmt.push_str(line);
        if stmt.ends_with(';') {
            warn!("Got stmt {}", stmt);
            // ignore query statements in schema
            if stmt.to_lowercase().contains("create") {
                if stmt.to_lowercase().contains("table") {
                    //let stmt = helpers::process_schema_stmt(&stmt, in_memory);
                    warn!("create_schema issuing stmt {}", stmt);
                    db.query_drop(stmt.to_string())?;
                } else {
                    db.query_drop(stmt.to_string())?;
                }
            }
            stmt = String::new();
        }
    }
    Ok(())
}

pub fn init_db(in_memory: bool, user: &str, pass: &str, host: &str, dbname: &str, schema: &str) {
    let url = format!("mysql://{}:{}@{}", user, pass, host);
    warn!("Init db {} url {}!", dbname, url);
    let mut db = mysql::Conn::new(Opts::from_url(&url).unwrap()).unwrap();
    warn!("Priming database");
    db.query_drop(&format!("DROP DATABASE IF EXISTS {};", dbname))
        .unwrap();
    db.query_drop(&format!("CREATE DATABASE {};", dbname))
        .unwrap();
    assert_eq!(db.ping(), true);
    assert_eq!(db.select_db(&format!("{}", dbname)), true);
    create_schema(&mut db, in_memory, schema).unwrap();
}

/************************************
 * MYSQL HELPERS
 ************************************/
pub fn query_drop(q: String, conn: &mut mysql::PooledConn) -> mysql::Result<()> {
    warn!("query_drop: {}\n", q);
    conn.query_drop(q)
}

pub fn query_drop_txn(q: String, txn: &mut mysql::Transaction) -> mysql::Result<()> {
    warn!("query_drop: {}\n", q);
    txn.query_drop(q)
}

pub fn escape_quotes_mysql(s: &str) -> String {
    let mut s = s.replace("\'", "\'\'");
    s = s.replace("\"", "\"\"");
    s
}

pub fn remove_escaped_chars(s: &str) -> String {
    let mut s = s.replace("\'\'", "\'");
    s = s.replace("\"\"", "\"");
    // hack to detect where there are empty strings
    // instead of escaped quotes...
    s = s.replace(":\"}", ":\"\"}");
    s = s.replace("\"\'\"", "\"\'\'\"");
    s
}

pub fn mysql_val_to_common_val(val: &mysql::Value) -> mysql_common::value::Value {
    match val {
        mysql::Value::NULL => mysql_common::value::Value::NULL,
        mysql::Value::Bytes(bs) => mysql_common::value::Value::Bytes(bs.clone()),
        mysql::Value::Int(i) => mysql_common::value::Value::Int(*i),
        mysql::Value::UInt(i) => mysql_common::value::Value::UInt(*i),
        mysql::Value::Float(f) => mysql_common::value::Value::Double((*f).into()),
        mysql::Value::Double(f) => mysql_common::value::Value::Double((*f).into()),
        mysql::Value::Date(a, b, c, d, e, f, g) => {
            mysql_common::value::Value::Date(*a, *b, *c, *d, *e, *f, *g)
        }
        mysql::Value::Time(a, b, c, d, e, f) => {
            mysql_common::value::Value::Time(*a, *b, *c, *d, *e, *f)
        }
    }
}

pub fn mysql_val_to_string(val: &mysql::Value) -> String {
    match val {
        mysql::Value::NULL => "NULL".to_string(),
        mysql::Value::Bytes(bs) => {
            let res = std::str::from_utf8(&bs);
            match res {
                Err(_) => String::new(),
                Ok(s) => remove_escaped_chars(s),
            }
        }
        mysql::Value::Int(i) => format!("{}", i),
        mysql::Value::UInt(i) => format!("{}", i),
        mysql::Value::Float(f) => format!("{}", f),
        _ => unimplemented!("No sqlparser support for dates yet?"), /*mysql::Date(u16, u8, u8, u8, u8, u8, u32),
                                                                    mysql::Time(bool, u32, u8, u8, u8, u32),8*/
    }
}

pub fn mysql_val_to_u64(val: &mysql::Value) -> mysql::Result<u64> {
    match val {
        mysql::Value::Bytes(bs) => {
            let res = std::str::from_utf8(&bs).unwrap();
            Ok(u64::from_str(res).unwrap())
        }
        mysql::Value::Int(i) => Ok(u64::from_str(&i.to_string()).unwrap()), // TODO fix?
        mysql::Value::UInt(i) => Ok(*i),
        _ => Err(mysql::Error::IoError(io::Error::new(
            io::ErrorKind::Other,
            format!("value {:?} is not an int", val),
        ))),
    }
}
