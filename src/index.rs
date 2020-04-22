use crate::client::{Client, ResponseMessage, SearchRequestMessage};
use crate::db::DB;
use crate::model::*;
use bson::{doc, oid::ObjectId, Document};
use mongodb::Database;
// use sonic_client::{ingest::IngestChan, search::SearchChan};
use tuikit::prelude::Result;

pub trait Index: ModelQuery + Tuples {}
impl Index<Spell> for Spell {}
