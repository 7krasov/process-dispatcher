use std::env;
pub struct EnvParams {
    http_port: u16,
    max_db_connections: u32,
    mvp_db_url: String,
    pd_db_url: String,
}

impl EnvParams {
    pub fn new(
        http_port: u16,
        max_db_connections: u32,
        mvp_db_url: String,
        pd_db_url: String,
    ) -> Self {
        EnvParams {
            http_port,
            max_db_connections,
            mvp_db_url,
            pd_db_url,
        }
    }

    pub fn http_port(&self) -> u16 {
        self.http_port
    }
    pub fn max_db_connections(&self) -> u32 {
        self.max_db_connections
    }
    pub fn mvp_db_url(&self) -> &str {
        &self.mvp_db_url
    }
    pub fn pd_db_url(&self) -> &str {
        &self.pd_db_url
    }
}

pub fn fetch_env_params() -> EnvParams {
    let http_port: u16 = match env::var("HTTP_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => {
            println!("HTTP_PORT is not set. Using default 8081");
            8081
        }
    };

    let max_db_connections: u32 = match env::var("MAX_DB_CONNECTIONS") {
        Ok(cnt) => cnt.parse::<u32>().unwrap(),
        Err(_) => {
            println!("MAX_DB_CONNECTIONS is not set. Using default 10");
            10
        }
    };

    let mvp_db_url: String = match env::var("MVP_DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            panic!("MVP_DATABASE_URL is not set");
        }
    };
    let pd_db_url: String = match env::var("PD_DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            panic!("PD_DATABASE_URL is not set");
        }
    };

    EnvParams::new(http_port, max_db_connections, mvp_db_url, pd_db_url)
}
