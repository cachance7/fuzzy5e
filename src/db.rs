use bson::Document;
use mongodb::error::Error;
use mongodb::{options::ClientOptions, Client, Database};

const DATABASE_NAME: &str = "5e-database";

pub struct DB {
    client: Client,
}

impl DB {
    pub fn connect(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Parse a connection string into an options struct.
        let url = format!("mongodb://{}/?connectTimeoutMS=2000&serverSelectionTimeoutMS=2000", addr);
        debug!("Url is {}", url);
        let client_options = ClientOptions::parse(&url)?;
        debug!("mongo parsed just fine");

        // Get a handle to the deployment.
        let client = Client::with_options(client_options)?;
        debug!("client is fine");

        Ok(DB { client })
    }
    pub fn with_db<F>(&self, f: F) -> Result<Vec<Document>, Error>
    where
        F: FnOnce(&Database) -> Result<Vec<Document>, Error>,
    {
        let db = self.client.database(DATABASE_NAME);
        let res = f(&db);
        drop(db);
        res
    }
}
