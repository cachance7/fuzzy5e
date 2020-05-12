use crate::client::{Client, IngestRequestMessage, ResponseMessage, SearchRequestMessage};
use crate::db::DB;
use crate::index::*;
use bson::{doc, oid::ObjectId, Document};
use mongodb::options::FindOptions;
use unicode_linebreak::{linebreaks, BreakOpportunity};
// use mongodb::Database;
use quick_error::quick_error;
use std::convert::{TryFrom, TryInto};
use tuikit::canvas;
use tuikit::prelude::*;

pub trait ScrollDraw {
    fn draw(&self, canvas: &mut dyn Canvas, scroll: usize) -> canvas::Result<()>;
}

#[derive(Debug, Clone)]
pub enum Model {
    Unknown(Document),
    Spell(Spell),
    Class(Class),
    Monster(Monster),
    Condition(Condition),
    MagicSchool(MagicSchool),
    Equipment(Equipment),
    Feature(Feature),
}

quick_error! {
    #[derive(Debug)]
    pub enum ModelError {
        NoInnerIndex(descr: &'static str) {
            display("Error {}", descr)
        }
    }
}

impl Model {
    fn inner_index(&self) -> Result<Box<dyn Index>> {
        match self {
            Self::Spell(m) => Ok(Box::new(m.clone())),
            Self::Class(m) => Ok(Box::new(m.clone())),
            Self::Monster(m) => Ok(Box::new(m.clone())),
            Self::Condition(m) => Ok(Box::new(m.clone())),
            Self::MagicSchool(m) => Ok(Box::new(m.clone())),
            Self::Equipment(m) => Ok(Box::new(m.clone())),
            Self::Feature(m) => Ok(Box::new(m.clone())),
            _ => Err(Box::new(ModelError::NoInnerIndex("Model"))),
        }
    }
}

pub trait DisplayName {
    fn display_name(&self) -> (String, Attr);
}

impl DisplayName for Model {
    fn display_name(&self) -> (String, Attr) {
        match self {
            Self::Spell(m) => m.display_name(),
            Self::Class(m) => m.display_name(),
            Self::Monster(m) => m.display_name(),
            Self::Condition(m) => m.display_name(),
            Self::MagicSchool(m) => m.display_name(),
            Self::Equipment(m) => m.display_name(),
            Self::Feature(m) => m.display_name(),
            Self::Unknown(m) => (
                String::from(m.get_str("name").unwrap_or("")),
                Attr::default(),
            ),
        }
    }
}

impl Model {
    pub fn draw(&self, canvas: &mut dyn Canvas, scroll: usize) -> canvas::Result<()> {
        match self {
            Model::Spell(m) => m.draw(canvas, scroll),
            Model::MagicSchool(m) => m.draw(canvas, scroll),
            Model::Monster(m) => m.draw(canvas, scroll),
            Model::Equipment(m) => m.draw(canvas, scroll),
            Model::Feature(m) => m.draw(canvas, scroll),
            Model::Condition(m) => m.draw(canvas, scroll),
            // Model::Class(m) => m.draw(canvas, scroll),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spell {
    pub id: String,
    pub name: String,
    pub desc: Vec<String>,
    document: Document,
}

pub trait Collection {
    fn collection() -> String;
}

pub trait ModelQuery {
    type Item: From<Document> + Collection + Index;

    /// Returns a cursor which will iterate over all items in the collection.
    fn all(d: &DB) -> Result<Vec<Box<Self::Item>>> {
        let all = doc! {};

        Self::find(d, all)
    }

    /// Performs a text search for the provided query. MongoDB requires that an index exist before
    /// using this.
    fn search(d: &DB, qs: &str) -> Result<Vec<Box<Self::Item>>> {
        let query = doc! {"$text": {"$search": qs}};

        Self::find(d, query)
    }

    /// Fetches all documents matching the provided MongoDB doc query.
    fn find(d: &DB, query: bson::Document) -> Result<Vec<Box<Self::Item>>> {
        let res = d
            .with_db(|db| {
                db.collection(&Self::Item::collection())
                    .find(query, FindOptions::builder().build())
                    .unwrap()
                    .collect()
            })
            .unwrap();
        Ok(res
            .iter()
            .map(|d: &Document| Box::new(Self::Item::from(d.clone())))
            .collect())
    }

    fn flush_all(c: impl Indexer) -> std::result::Result<(), IndexError> {
        c.flush_all(&Self::Item::collection())
    }

    /// Indexes all items in the collection. For speed purposes, this does not wait for
    /// confirmation before continuing.
    fn index_all(s: impl Indexer, db: &DB) -> std::result::Result<(), IndexError> {
        s.index_bulk(Self::all(db).unwrap())
    }

    fn indexed_query(s: impl Indexer, db: &DB, qs: &str) -> Result<Vec<Box<Self::Item>>> {
        let ids = s.query(&Self::Item::collection(), qs)?;

        // heymywife is the best wife she's so hot and fun and smart and loevly and I'm the best too for making her drinks}

        let oids: Vec<ObjectId> = ids
            .iter()
            .map(|sid| bson::oid::ObjectId::with_string(sid).unwrap())
            .collect();

        let query: Document = doc! {"_id": {"$in": oids}};

        Self::find(db, query)
    }
}

impl Collection for Spell {
    fn collection() -> String {
        String::from("spells")
    }
}

impl Index for Spell {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn tuples(&self) -> Vec<(String, String, String, String)> {
        let mut t: Vec<(String, String, String, String)> = Vec::default();
        t.push((
            Self::collection(),
            String::from("name"),
            self.id(),
            self.name.clone(),
        ));
        for d in &self.desc {
            t.push((
                Self::collection(),
                String::from("desc"),
                self.id(),
                d.to_string(),
            ));
        }
        t
    }
}

impl Collection for Monster {
    fn collection() -> String {
        String::from("monsters")
    }
}

impl Index for Monster {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn tuples(&self) -> Vec<(String, String, String, String)> {
        let mut t: Vec<(String, String, String, String)> = Vec::default();
        t.push((
            Self::collection(),
            String::from("name"),
            self.id(),
            self.name.clone(),
        ));
        t.push((
            Self::collection(),
            String::from("type"),
            self.id(),
            self.name.clone(),
        ));
        t
    }
}

impl Collection for Class {
    fn collection() -> String {
        String::from("classes")
    }
}

impl Index for Class {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn tuples(&self) -> Vec<(String, String, String, String)> {
        let mut t: Vec<(String, String, String, String)> = Vec::default();
        t.push((
            Self::collection(),
            String::from("name"),
            self.id(),
            self.name.clone(),
        ));
        t
    }
}

impl Collection for Condition {
    fn collection() -> String {
        String::from("conditions")
    }
}

impl Index for Condition {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn tuples(&self) -> Vec<(String, String, String, String)> {
        let mut t: Vec<(String, String, String, String)> = Vec::default();
        t.push((
            Self::collection(),
            String::from("name"),
            self.id(),
            self.name.clone(),
        ));
        for d in &self.desc {
            t.push((
                Self::collection(),
                String::from("desc"),
                self.id(),
                d.to_string(),
            ));
        }
        t
    }
}

impl ModelQuery for Spell {
    type Item = Spell;
}

impl ModelQuery for Condition {
    type Item = Condition;
}

impl ModelQuery for Monster {
    type Item = Monster;
}

impl ModelQuery for Class {
    type Item = Class;
}

impl ModelQuery for MagicSchool {
    type Item = MagicSchool;
}

impl ModelQuery for Equipment {
    type Item = Equipment;
}

impl ModelQuery for Feature {
    type Item = Feature;
}

impl Collection for Model {
    fn collection() -> String {
        String::from("all")
    }
}

/// This trait implementation is for dealing with models at the aggregate level.
/// In other words, id() will produce ids of the form <collection>:<objectid>
/// depending on the enum type.
///
/// Example:
///
/// ```no_run
/// let m = Model::Spell(Spell::new("12345", "Wizard Touch", vec!["Touches a wizard"]));
/// assert_eq("spells:12345", m.id());
/// match m {
///     Model::Spell(s) => assert_eq("12345", s.id()),
///     _ => unreachable!(),
/// }
/// ```
impl Index for Model {
    fn id(&self) -> String {
        match self {
            Self::Spell(m) => format!("{}:{}", Spell::collection(), m.id()),
            Self::Monster(m) => format!("{}:{}", Monster::collection(), m.id()),
            Self::Class(m) => format!("{}:{}", Class::collection(), m.id()),
            Self::Condition(m) => format!("{}:{}", Condition::collection(), m.id()),
            Self::MagicSchool(m) => format!("{}:{}", MagicSchool::collection(), m.id()),
            Self::Equipment(m) => format!("{}:{}", Equipment::collection(), m.id()),
            Self::Feature(m) => format!("{}:{}", Feature::collection(), m.id()),
            Self::Unknown(m) => String::from(m.get_str("id").unwrap_or("")),
        }
    }

    fn tuples(&self) -> Vec<(String, String, String, String)> {
        self.inner_index()
            .unwrap()
            .tuples()
            .iter()
            .map(|t| (Self::collection(), t.1.clone(), self.id(), t.3.clone()))
            .collect()
    }
}

/// TODO I don't think this implementation is actually used anywhere...
impl From<Document> for Model {
    fn from(d: Document) -> Self {
        Self::Unknown(d)
    }
}

type ModelQueryFn = Box<dyn (Fn(&DB) -> Vec<Box<Model>>) + Send + 'static>;

impl ModelQuery for Model {
    type Item = Model;

    /// Flushes _all_ collections from the index.
    fn flush_all(c: impl Indexer) -> std::result::Result<(), IndexError> {
        Spell::flush_all(c.clone())?;
        Monster::flush_all(c.clone())?;
        Condition::flush_all(c.clone())?;
        Class::flush_all(c.clone())?;
        MagicSchool::flush_all(c.clone())?;
        Equipment::flush_all(c.clone())?;
        Feature::flush_all(c.clone())?;
        c.flush_all(&Self::Item::collection())
    }

    /// This index_all variant pulls the results into a global collection "all".
    /// Each object will be prefixed with its collection. The intent of this is
    /// to allow searching across all indexed object types at the same time and
    /// allowing sonic to return the most relevant ones.
    ///
    /// Example:
    ///
    /// # item-specific push
    /// > PUSH spells name 12345 "magic fireball"
    ///
    /// # global push
    /// > PUSH all name spells:12345 "magic fireball"
    ///
    fn index_all(s: impl Indexer, database: &DB) -> std::result::Result<(), IndexError> {
        let fns: Vec<ModelQueryFn> = vec![
            Box::new(|db| {
                Spell::all(db)
                    .unwrap()
                    .iter()
                    .map(|s| Box::new(Model::Spell(*s.clone())))
                    .collect()
            }),
            Box::new(|db| {
                Monster::all(db)
                    .unwrap()
                    .iter()
                    .map(|s| Box::new(Model::Monster(*s.clone())))
                    .collect()
            }),
            Box::new(|db| {
                Class::all(db)
                    .unwrap()
                    .iter()
                    .map(|s| Box::new(Model::Class(*s.clone())))
                    .collect()
            }),
            Box::new(|db| {
                Condition::all(db)
                    .unwrap()
                    .iter()
                    .map(|s| Box::new(Model::Condition(*s.clone())))
                    .collect()
            }),
            Box::new(|db| {
                MagicSchool::all(db)
                    .unwrap()
                    .iter()
                    .map(|s| Box::new(Model::MagicSchool(*s.clone())))
                    .collect()
            }),
            Box::new(|db| {
                Equipment::all(db)
                    .unwrap()
                    .iter()
                    .map(|s| Box::new(Model::Equipment(*s.clone())))
                    .collect()
            }),
            Box::new(|db| {
                Feature::all(db)
                    .unwrap()
                    .iter()
                    .map(|s| Box::new(Model::Feature(*s.clone())))
                    .collect()
            }),
        ];
        for f in fns {
            s.index_bulk(f(database));
            // for model in f(database) {
            //     // Indexes into the global collection
            //     s.index(Box::new(model));
            //     match model {
            //         Self::Spell(m) => Ok(Box::new(&m.clone())),
            //         Self::Class(m) => Ok(Box::new(&m.clone())),
            //         Self::Monster(m) => Ok(Box::new(&m.clone())),
            //         Self::Condition(m) => Ok(Box::new(&m.clone())),
            //         Self::MagicSchool(m) => Ok(Box::new(&m.clone())),
            //         Self::Equipment(m) => Ok(Box::new(&m.clone())),
            //         Self::Feature(m) => Ok(Box::new(&m.clone())),
            //         _ => Err(Box::new(ModelError::NoInnerIndex("Model"))),
            //     }
            // }
        }
        Ok(())
    }

    /// Implementation for Model enum. This performs a query across all types.
    fn indexed_query(s: impl Indexer, db: &DB, qs: &str) -> Result<Vec<Box<Self::Item>>> {
        let ids = s.query(&Self::Item::collection(), qs)?;

        let sids: Vec<(&str, &str)> = ids
            .iter()
            .map(|r: &String| -> (&str, &str) {
                let s: Vec<&str> = r.split(':').collect();
                (s.get(0).unwrap(), s.get(1).unwrap())
            })
            .collect();

        //FIXME This is a very stupid implementation
        let oids: Vec<ObjectId> = sids
            .iter()
            .map(|(_, sid)| bson::oid::ObjectId::with_string(sid).unwrap())
            .collect();

        let query: Document = doc! {"_id": {"$in": oids}};

        let mut mresults = std::collections::HashMap::new();
        let mut results: Vec<Model> = Vec::default();

        results.extend(
            Spell::find(db, query.clone())?
                .iter()
                .map(|m| Model::Spell(*m.clone())),
        );
        results.extend(
            Monster::find(db, query.clone())?
                .iter()
                .map(|m| Model::Monster(*m.clone())),
        );
        results.extend(
            Class::find(db, query.clone())?
                .iter()
                .map(|m| Model::Class(*m.clone())),
        );
        results.extend(
            Condition::find(db, query.clone())?
                .iter()
                .map(|m| Model::Condition(*m.clone())),
        );
        results.extend(
            MagicSchool::find(db, query.clone())?
                .iter()
                .map(|m| Model::MagicSchool(*m.clone())),
        );
        results.extend(
            Equipment::find(db, query.clone())?
                .iter()
                .map(|m| Model::Equipment(*m.clone())),
        );
        results.extend(
            Feature::find(db, query.clone())?
                .iter()
                .map(|m| Model::Feature(*m.clone())),
        );

        for r in results {
            mresults.insert(r.id(), r);
        }

        let mut ordered_results = Vec::new();
        for id in ids {
            if let Some(r) = mresults.get(&id) {
                ordered_results.push(Box::new(r.clone()));
            }
        }

        Ok(ordered_results)
    }
}

macro_rules! impl_From {
    (for $($t:ident),+) => {
        $(impl From<Document> for $t {
            fn from(d: Document) -> Self {
                $t {
                    id: String::from(d.get_object_id("_id").unwrap().to_hex()),
                    name: String::from(d.get_str("name").unwrap_or("")),
                    desc: d
                        .get_array("desc")
                        .unwrap_or(&Vec::default())
                        .iter()
                        .map(|x| String::from(x.as_str().unwrap()))
                        .collect(),
                    document: d,
                }
            }
        })*
    }
}
fn get_break_idx(line: &str, width: usize) -> usize {
    if line.len() < width {
        return line.len();
    }

    let breaks = linebreaks(line);
    let mut break_idx: i32 = -1;
    for b in breaks.clone() {
        if b.0 < width {
            break_idx += 1;
        } else {
            break;
        }
    }
    if break_idx == -1 {
        std::cmp::min(width, line.len())
    } else {
        let b: Vec<(usize, BreakOpportunity)> = breaks.collect();
        b.get(usize::try_from(break_idx).unwrap()).unwrap().0
    }
}

fn break_at(longline: &str, width: usize) -> Vec<&str> {
    trace!("break_at");
    let mut v = Vec::new();
    let mut break_at = get_break_idx(&longline, width);
    let (mut l, mut rest) = longline.split_at(break_at);
    v.push(l);
    while !rest.is_empty() {
        break_at = get_break_idx(&rest, width);
        let s = rest.split_at(break_at);
        l = s.0;
        rest = s.1;
        v.push(l);
    }
    v
}

fn format_usage(d: &Document) -> String {
    if let Ok(utype) = d.get_str("type") {
        match utype {
            "recharge after rest" => format!(
                "(Recharges after a {} rest)",
                d.get_array("rest_types")
                    .unwrap()
                    .iter()
                    .map(|r| r.as_str().unwrap())
                    .collect::<Vec<&str>>()
                    .join(" or ")
            ),
            "recharge on roll" => format!(
                "(Recharge {}, {}+)",
                d.get_i32("min_val").unwrap_or_default(),
                d.get_str("dice").unwrap_or_default()
            ),
            "per day" => format!("({}/day)", d.get_i32("times").unwrap_or_default()),
            _ => String::default(),
        }
    } else {
        String::default()
    }
}

fn draw_actions(
    canvas: &mut dyn Canvas,
    actions: &[bson::Bson],
    width: usize,
    row: i32,
) -> canvas::Result<i32> {
    let mut idx = 0;
    for s in actions {
        let starting_row = row + idx;
        if let Some(d) = s.as_document() {
            // Build line we'll be wrapping
            let mut line = String::default();
            if let Ok(name) = d.get_str("name") {
                line.push_str(&name);
                if let Ok(usage) = d.get_document("usage") {
                    line.push_str(&format!(" {}", format_usage(usage)));
                }
                line.push_str(". ");
            }
            let name = line.clone();

            if let Ok(desc) = d.get_str("desc") {
                line.push_str(desc);
                for l in break_at(&line, width) {
                    idx += print(canvas, row + idx, 0, l, Attr::default()).unwrap();
                }

                let _ = print(canvas, starting_row, 0, &name, Attr::from(Effect::BOLD));
            }
            idx += 1;
        }
    }
    Ok(idx)
}

impl ScrollDraw for Spell {
    fn draw(&self, canvas: &mut dyn Canvas, scroll: usize) -> canvas::Result<()> {
        let (width, _height) = canvas.size()?;
        let col = 0;
        let mut row: i32 = -(i32::try_from(scroll).unwrap());
        row += print(canvas, row, col, &self.name, Attr {fg: Color::AnsiValue(202), effect: Effect::BOLD, ..Attr::default()}).unwrap();

        if let Ok(level) = self.document.get_i32("level") {
            let level = if level == 0 {
                String::from("cantrip")
            } else {
                format!("Level {}", level)
            };
            if let Ok(school) = self.document.get_document("school") {
                let school_name = school.get_str("name").unwrap();
                row += print(
                    canvas,
                    row,
                    col,
                    &format!("{} {}", level, school_name),
                    Attr::default(),
                )
                .unwrap();
            }
        }

        row += 1;

        if let Ok(casting_time) = self.document.get_str("casting_time") {
            row += print_with_title(canvas, row, col, width, casting_time, Some("Casting Time:"))
                .unwrap();
        }

        if let Ok(range) = self.document.get_str("range") {
            row += print_with_title(canvas, row, col, width, range, Some("Range:")).unwrap();
        }

        if let Ok(components) = self.document.get_array("components") {
            if !components.is_empty() {
                let mut s = String::default();

                s.push_str(&components
                        .iter()
                        .map(|v| v.as_str().unwrap())
                        .collect::<Vec<&str>>()
                        .join(", "));
                if let Ok(material) = self.document.get_str("material") {
                    s.push_str(&format!(" ({})", material));
                }

                row += print_with_title(
                    canvas,
                    row,
                    col,
                    width,
                    &s,
                    Some("Components:"),
                )
                .unwrap()
            }
        }

        if let Ok(duration) = self.document.get_str("duration") {
            let mut s = String::default();
            if let Ok(concentration) = self.document.get_bool("concentration"){
                if concentration {
                    s.push_str("Concentration, ");
                }
            }
            s.push_str(duration);
            row += print_with_title(canvas, row, col, width, &s, Some("Duration:")).unwrap()
        }

        if let Ok(classes) = self.document.get_array("classes") {
            row += print_with_title(
                canvas,
                row,
                col,
                width,
                &classes
                    .iter()
                    .map(|c| c.as_document().unwrap().get_str("name").unwrap())
                    .collect::<Vec<&str>>()
                    .join(", "),
                Some("Classes:"),
            )
            .unwrap();
        }

        row += 1;

        for line in &self.desc {
            for l in break_at(line, width) {
                let _ = print(canvas, row, col, &l, Attr::default());
                row += 1;
            }
        }

        if let Ok(higher_level) = self.document.get_array("higher_level") {
            row += print_with_title(
                canvas,
                row,
                col,
                width,
                &higher_level
                    .iter()
                    .map(|c| c.as_str().unwrap())
                    .collect::<Vec<&str>>()
                    .join(" "),
                Some("At Higher Levels:"),
            )
            .unwrap()
        }

        Ok(())
    }
}

// impl Widget for Spell {}

fn print(
    canvas: &mut dyn Canvas,
    row: i32,
    col: usize,
    text: &str,
    attr: Attr,
) -> canvas::Result<i32> {
    if row >= 0 {
        let _ = canvas.print_with_attr(row.try_into().unwrap(), col, text, attr);
    }
    Ok(1)
}

fn print_with_title(
    canvas: &mut dyn Canvas,
    row: i32,
    col: usize,
    width: usize,
    text: &str,
    title: Option<&str>,
) -> canvas::Result<i32> {
    let mut idx = 0;
    let mut s = String::default();
    if let Some(t) = title {
        s.push_str(&format!("{} ", t));
    }
    s.push_str(text);
    for l in break_at(&s, width) {
        idx += print(canvas, row + idx, col, l, Attr::default()).unwrap();
    }
    if let Some(t) = title {
        let _ = print(canvas, row, col, t, Attr::from(Effect::BOLD)).unwrap();
    }

    Ok(idx)
}

fn calc_modifier(val: i32) -> i32 {
    (val - 10) / 2
}

/// Formats a single stat
fn format_stat(val: i32) -> String {
    let modifier = calc_modifier(val);
    let sign = if modifier >= 0 { "+" } else { "" };
    let stat_fmt = format!("{}({}{})", val, sign, modifier);
    if stat_fmt.len() == 5 {
        format!("  {}  ", stat_fmt)
    } else if stat_fmt.len() == 6 {
        format!("  {} ", stat_fmt)
    } else if stat_fmt.len() == 7 {
        format!(" {} ", stat_fmt)
    } else {
        unreachable!();
    }
}

//        ___STR___   DEX      CON      INT      WIS      CHA
//          27(+8)  14(+12)   5(+0)      16(+3)      15(+2)      19(+4)

/// Monster stat blocks get up to two panels.
/// This is the draw fn for left block
impl ScrollDraw for Monster {
    fn draw(&self, canvas: &mut dyn Canvas, scroll: usize) -> canvas::Result<()> {
        let (width, _height) = canvas.size()?;
        let col = 0;
        let mut row: i32 = -(i32::try_from(scroll).unwrap());

        // Name
        row += print(
            canvas,
            row,
            col,
            &self.name,
            Attr {
                effect: Effect::BOLD,
                ..Attr::default()
            },
        )
        .unwrap();

        // Size Type, Alignment
        row += print(
            canvas,
            row,
            col,
            &format!(
                "{} {}, {}",
                self.document.get_str("size").unwrap_or_default(),
                self.document.get_str("type").unwrap_or_default(),
                self.document.get_str("alignment").unwrap_or_default()
            ),
            Attr::default(),
        )
        .unwrap();

        // ---
        row += print(
            canvas,
            row,
            col,
            &"~".repeat(width),
            Attr {
                effect: Effect::BOLD,
                ..Attr::default()
            },
        )
        .unwrap();

        // AC, HP, Speed
        if let Ok(ac) = self.document.get_i32("armor_class") {
            row += print_with_title(
                canvas,
                row,
                col,
                width,
                &format!("{}", ac,),
                Some("Armor Class:"),
            )
            .unwrap();
        }

        // Hit Points
        if let Ok(hp) = self.document.get_i32("hit_points") {
            row += print_with_title(
                canvas,
                row,
                col,
                width,
                &format!("{}", hp,),
                Some("Hit Points:"),
            )
            .unwrap();
        }

        // Speed
        if let Ok(speed) = self.document.get_document("speed") {
            let mut s = Vec::default();
            if let Ok(walk) = speed.get_str("walk") {
                s.push(String::from(walk));
            }
            if let Ok(climb) = speed.get_str("climb") {
                s.push(format!("climb {}", climb));
            }
            if let Ok(swim) = speed.get_str("swim") {
                s.push(format!("swim {}", swim));
            }
            if let Ok(fly) = speed.get_str("fly") {
                s.push(format!("fly {}", fly));
            }
            row +=
                print_with_title(canvas, row, col, width, &s.join(", "), Some("Speed:")).unwrap();
        }

        // ---
        row += print(
            canvas,
            row,
            col,
            &"~".repeat(width),
            Attr {
                effect: Effect::BOLD,
                ..Attr::default()
            },
        )
        .unwrap();

        // STR, DEX, CON, INT, WIS, CHA
        let stats = vec!["STR", "DEX", "CON", "INT", "WIS", "CHA"];
        let spacing = 6;
        row += print(canvas, row, col, &(" ".repeat(spacing/2) + &stats.join(&" ".repeat(spacing))), Attr::from(Effect::BOLD)).unwrap();

        let attrs = vec!["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"];
        let mut s = Vec::default();
        for a in attrs {
            if let Ok(val) = self.document.get_i32(a) {
                s.push(format_stat(val));
            }
        }

        row += print(canvas, row, col, &(" ".repeat(spacing/2-3) + &s.join("")), Attr::default()).unwrap();

        // ---
        row += print(
            canvas,
            row,
            col,
            &"~".repeat(width),
            Attr {
                effect: Effect::BOLD,
                ..Attr::default()
            },
        )
        .unwrap();

        // Proficiencies
        if let Ok(profs) = self.document.get_array("proficiencies") {
            let saves = profs
                .iter()
                .filter_map(|doc| {
                    if let bson::Bson::Document(d) = doc {
                        if d.get_str("name").unwrap_or_default().starts_with("Saving") {
                            Some(format!(
                                "{} +{}",
                                d.get_str("name")
                                    .unwrap_or_default()
                                    .rsplit(' ')
                                    .collect::<Vec<&str>>()
                                    .get(0)
                                    .unwrap(),
                                d.get_i32("value").unwrap_or_default()
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
                .join(", ");

            if !saves.is_empty() {
                row += print_with_title(canvas, row, col, width, &saves, Some("Saving Throws:"))
                    .unwrap();
            }
            let skills = profs
                .iter()
                .filter_map(|doc| {
                    if let bson::Bson::Document(d) = doc {
                        if d.get_str("name").unwrap_or_default().starts_with("Skill") {
                            Some(format!(
                                "{} +{}",
                                d.get_str("name")
                                    .unwrap_or_default()
                                    .split(' ')
                                    .collect::<Vec<&str>>()
                                    .get(1)
                                    .unwrap(),
                                d.get_i32("value").unwrap_or_default()
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
                .join(", ");
            if !skills.is_empty() {
                row += print_with_title(canvas, row, col, width, &skills, Some("Skills:")).unwrap();
            }
        }

        // Vuln + Res + Immun
        if let Ok(vuln) = self.document.get_array("damage_vulnerabilities") {
            if !vuln.is_empty() {
                row += print_with_title(
                    canvas,
                    row,
                    col,
                    width,
                    &vuln
                        .iter()
                        .map(|v| v.as_str().unwrap())
                        .collect::<Vec<&str>>()
                        .join(", "),
                    Some("Damage Vulnerabilities:"),
                )
                .unwrap();
            }
        }

        if let Ok(res) = self.document.get_array("damage_resistances") {
            if !res.is_empty() {
                row += print_with_title(
                    canvas,
                    row,
                    col,
                    width,
                    &res.iter()
                        .map(|v| v.as_str().unwrap())
                        .collect::<Vec<&str>>()
                        .join(", "),
                    Some("Damage Resistances:"),
                )
                .unwrap();
            }
        }

        if let Ok(immun) = self.document.get_array("damage_immunities") {
            if !immun.is_empty() {
                row += print_with_title(
                    canvas,
                    row,
                    col,
                    width,
                    &immun
                        .iter()
                        .map(|v| v.as_str().unwrap())
                        .collect::<Vec<&str>>()
                        .join(", "),
                    Some("Damage Immunities:"),
                )
                .unwrap();
            }
        }

        if let Ok(cond) = self.document.get_array("condition_immunities") {
            if !cond.is_empty() {
                row += print_with_title(
                    canvas,
                    row,
                    col,
                    width,
                    &cond
                        .iter()
                        .map(|v| v.as_document().unwrap().get_str("name").unwrap())
                        .collect::<Vec<&str>>()
                        .join(", "),
                    Some("Condition Immunities:"),
                )
                .unwrap();
            }
        }

        // Senses
        if let Ok(senses) = self.document.get_document("senses") {
            let mut s = String::default();
            if let Ok(truesight) = senses.get_str("truesight") {
                s.push_str(&format!("truesight {}, ", truesight));
            }
            if let Ok(blindsight) = senses.get_str("blindsight") {
                s.push_str(&format!("blindsight {}, ", blindsight));
            }
            if let Ok(darkvision) = senses.get_str("darkvision") {
                s.push_str(&format!("darkvision {}, ", darkvision));
            }
            if let Ok(pp) = senses.get_i32("passive_perception") {
                s.push_str(&format!("passive Perception {}", pp));
            }

            row += print_with_title(canvas, row, col, width, &s, Some("Senses:")).unwrap();
        }

        // Languages
        if let Ok(languages) = self.document.get_str("languages") {
            if !languages.is_empty() {
                row += print_with_title(canvas, row, col, width, languages, Some("Languages:"))
                    .unwrap();
            }
        }

        // CR
        if let Ok(cr) = self.document.get_i32("challenge_rating") {
            row += print_with_title(
                canvas,
                row,
                col,
                width,
                &format!("{}", cr),
                Some("Challenge:"),
            )
            .unwrap();
        }

        // ---
        row += print(
            canvas,
            row,
            col,
            &"~".repeat(width),
            Attr {
                effect: Effect::BOLD,
                ..Attr::default()
            },
        )
        .unwrap();

        // Special abilities
        if let Ok(special) = self.document.get_array("special_abilities") {
            row += draw_actions(canvas, special, width, row).unwrap();
        }

        // Actions
        if let Ok(actions) = self.document.get_array("actions") {
            row += print(
                canvas,
                row,
                col,
                "Actions",
                Attr {
                    effect: Effect::BOLD,
                    fg: Color::RED,
                    ..Attr::default()
                },
            )
            .unwrap();
            row += draw_actions(canvas, actions, width, row).unwrap();
        }

        // Legendary Actions
        if let Ok(actions) = self.document.get_array("legendary_actions") {
            row += print(
                canvas,
                row,
                col,
                "Legendary Actions",
                Attr {
                    effect: Effect::BOLD,
                    fg: Color::RED,
                    ..Attr::default()
                },
            )
            .unwrap();
            row += draw_actions(canvas, actions, width, row).unwrap();
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Class {
    id: String,
    name: String,
    desc: Vec<String>,
    document: Document,
}

#[derive(Debug, Clone)]
pub struct Monster {
    id: String,
    name: String,
    desc: Vec<String>,
    document: Document,
}

#[derive(Debug, Clone)]
pub struct Race {
    id: String,
    name: String,
    desc: Vec<String>,
    document: Document,
}

#[derive(Debug, Clone)]
pub struct Subclass {
    id: String,
    name: String,
    desc: Vec<String>,
    document: Document,
}

#[derive(Debug, Clone)]
pub struct Condition {
    id: String,
    name: String,
    desc: Vec<String>,
    document: Document,
}

#[derive(Debug, Clone)]
pub struct MagicSchool {
    pub id: String,
    pub name: String,
    pub desc: String,
    document: Document,
}

#[derive(Debug, Clone)]
pub struct Equipment {
    pub id: String,
    pub name: String,
    // desc: Vec<String>,
    document: Document,
}

#[derive(Debug, Clone)]
pub struct Feature {
    id: String,
    name: String,
    desc: Vec<String>,
    document: Document,
}

impl Collection for MagicSchool {
    fn collection() -> String {
        String::from("magic-schools")
    }
}

impl Index for MagicSchool {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn tuples(&self) -> Vec<(String, String, String, String)> {
        let mut t: Vec<(String, String, String, String)> = Vec::default();
        t.push((
            Self::collection(),
            String::from("name"),
            self.id(),
            self.name.clone(),
        ));
        t.push((
            Self::collection(),
            String::from("desc"),
            self.id(),
            self.desc.clone(),
        ));
        t
    }
}

impl From<Document> for MagicSchool {
    fn from(d: Document) -> Self {
        Self {
            id: d.get_object_id("_id").unwrap().to_hex(),
            name: String::from(d.get_str("name").unwrap_or("")),
            desc: String::from(d.get_str("desc").unwrap_or_default()),
            document: d,
        }
    }
}

impl ScrollDraw for MagicSchool {
    fn draw(&self, canvas: &mut dyn Canvas, scroll: usize) -> canvas::Result<()> {
        let (width, _height) = canvas.size()?;
        let col = 0;
        let mut row: i32 = -(i32::try_from(scroll).unwrap());
        let _ = print(canvas, row, col, &self.name, Attr::from(Effect::BOLD));
        row += 1;

        trace!("drawing");
        let _ = print_with_title(canvas, row, col, width, &self.desc, None);
        row += 1;

        Ok(())
    }
}

impl From<Document> for Equipment {
    fn from(d: Document) -> Self {
        Self {
            id: d.get_object_id("_id").unwrap().to_hex(),
            name: String::from(d.get_str("name").unwrap_or("")),
            document: d,
        }
    }
}

impl Collection for Equipment {
    fn collection() -> String {
        String::from("equipment")
    }
}

impl Index for Equipment {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn tuples(&self) -> Vec<(String, String, String, String)> {
        let mut t: Vec<(String, String, String, String)> = Vec::default();
        t.push((
            Self::collection(),
            String::from("name"),
            self.id(),
            self.name.clone(),
        ));

        if let Ok(dmg) = self.document.get_document("damage_type") {
            if let Ok(name) = dmg.get_str("name") {
                t.push((
                    Self::collection(),
                    String::from("desc"),
                    self.id(),
                    String::from(name),
                ));
            }
        }
        if let Ok(properties) = self.document.get_array("properties") {
            for p in properties {
                t.push((
                    Self::collection(),
                    String::from("desc"),
                    self.id(),
                    String::from(p.as_document().unwrap().get_str("name").unwrap()),
                ));
            }
        }
        t
    }
}

impl ScrollDraw for Equipment {
    fn draw(&self, canvas: &mut dyn Canvas, scroll: usize) -> canvas::Result<()> {
        let (width, _height) = canvas.size()?;
        let col = 0;
        let mut row: i32 = -(i32::try_from(scroll).unwrap());
        row += print(canvas, row, col, &self.name, Attr::from(Effect::BOLD)).unwrap();

        if let Ok(dmg) = self.document.get_document("damage") {
            let mut d = String::default();
            if let Ok(dice) = dmg.get_str("damage_dice") {
                d.push_str(dice);
            }
            if let Ok(bonus) = dmg.get_i32("damage_bonus") {
                if bonus > 0 {
                    d.push_str(&format!("+{} ", bonus));
                } else {
                    d.push(' ');
                }
            }
            if let Ok(dt) = dmg.get_document("damage_type") {
                d.push_str(dt.get_str("name").unwrap());
            }
            print(canvas, row, col, &d, Attr::default())?;
        }

        Ok(())
    }
}

impl Collection for Feature {
    fn collection() -> String {
        String::from("features")
    }
}

impl Index for Feature {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn tuples(&self) -> Vec<(String, String, String, String)> {
        let mut t: Vec<(String, String, String, String)> = Vec::default();
        t.push((
            Self::collection(),
            String::from("name"),
            self.id(),
            self.name.clone(),
        ));
        for d in &self.desc {
            t.push((
                Self::collection(),
                String::from("desc"),
                self.id(),
                d.to_string(),
            ));
        }
        t
    }
}

impl ScrollDraw for Feature {
    fn draw(&self, canvas: &mut dyn Canvas, scroll: usize) -> canvas::Result<()> {
        let (width, _height) = canvas.size()?;
        let col = 0;
        let mut row: i32 = -(i32::try_from(scroll).unwrap());
        row += print(canvas, row, col, &self.name, Attr::from(Effect::BOLD))?;
        if let Ok(class) = self.document.get_document("class") {
            let mut s = Vec::default();
            if let Ok(level) = self.document.get_i32("level") {
                s.push(format!("Level {}", level));
            }
            if let Ok(name) = class.get_str("name") {
                s.push(String::from(name));
            }
            if let Ok(subclass) = self.document.get_document("subclass") {
                if let Ok(name) = subclass.get_str("name") {
                    s.push(format!("({})", name));
                }
            }
            row += print(canvas, row, col, &s.join(" "), Attr::default())?;
        }
        row += 1;

        for line in &self.desc {
            for l in break_at(line, width) {
                let _ = print(canvas, row, col, &l, Attr::default());
                row += 1;
            }
        }
        row += 1;

        Ok(())
    }
}

impl ScrollDraw for Condition {
    fn draw(&self, canvas: &mut dyn Canvas, scroll: usize) -> canvas::Result<()> {
        let (width, _height) = canvas.size()?;
        let col = 0;
        let mut row: i32 = -(i32::try_from(scroll).unwrap());
        row += print(canvas, row, col, &self.name, Attr::from(Effect::BOLD))?;

        for line in &self.desc {
            for l in break_at(line, width) {
                let _ = print(canvas, row, col, &l, Attr::default());
                row += 1;
            }
        }
        row += 1;

        Ok(())
    }
}

impl DisplayName for Spell {
    fn display_name(&self) -> (String, Attr) {
        (format!("🔮 {}", self.name), Attr::from(Color::MAGENTA))
    }
}
impl DisplayName for Condition {
    fn display_name(&self) -> (String, Attr) {
        (format!("💢 {}", self.name), Attr::default())
    }
}
impl DisplayName for Class {
    fn display_name(&self) -> (String, Attr) {
        (format!("👤 {}", self.name), Attr::from(Color::BLUE))
    }
}
impl DisplayName for Monster {
    fn display_name(&self) -> (String, Attr) {
        (format!("👹 {}", self.name), Attr::from(Color::RED))
    }
}
impl DisplayName for MagicSchool {
    fn display_name(&self) -> (String, Attr) {
        (
            format!(" {}", self.document.get_str("name").unwrap()),
            Attr::from(Color::GREEN),
        )
    }
}
impl DisplayName for Equipment {
    fn display_name(&self) -> (String, Attr) {
        (
            format!("🏹 {}", self.document.get_str("name").unwrap()),
            Attr::from(Color::LIGHT_YELLOW),
        )
    }
}
impl DisplayName for Feature {
    fn display_name(&self) -> (String, Attr) {
        (
            format!("💡 {}", self.document.get_str("name").unwrap()),
            Attr::from(Color::LIGHT_GREEN),
        )
    }
}

impl_From!(for Spell, Monster, Condition, Class, Subclass, Race, Feature);
