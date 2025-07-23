use std::io::{Read, Write};
use thiserror::Error;

// TODO: i think this method of doing error handling probably has bad scoping
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ParseError(#[from] chrono::ParseError),
}

pub trait Enterable
where
    Self: Sized,
{
    // load the type from a directory path
    // TODO: return a result
    fn fetch_from_fs(path: &std::path::Path) -> Result<Self, Error>;

    // store self in the FS
    // TODO: return a result
    fn store_to_fs(self, path: &std::path::Path) -> Result<(), Error>;
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Entry<T: Enterable> {
    pub id: String,
    pub author: String,
    pub creation_time: chrono::DateTime<chrono::Local>,
    pub entry: T,
}

impl<T: Enterable> Enterable for Entry<T> {
    fn fetch_from_fs(path: &std::path::Path) -> Result<Self, Error> {
        let id_file_path = path.join("id");
        let mut id_file = std::fs::File::open(&id_file_path)?;
        let mut id_buffer = String::new();
        id_file.read_to_string(&mut id_buffer)?;

        // author
        let author_file_path = path.join("author");
        let mut author_file = std::fs::File::open(&author_file_path)?;
        let mut author_buffer = String::new();
        author_file.read_to_string(&mut author_buffer)?;

        // creation_time
        let creation_time_file_path = path.join("creation_time");
        let mut creation_time_file = std::fs::File::open(&creation_time_file_path)?;
        let mut creation_time_buffer = String::new();
        creation_time_file.read_to_string(&mut creation_time_buffer)?;

        let entry_file_path_buf = path.join("entry");
        let entry_file_path = entry_file_path_buf.as_path();

        Ok(Self {
            id: id_buffer,
            author: author_buffer,
            creation_time: chrono::DateTime::<_>::parse_from_rfc3339(&creation_time_buffer)?.into(),
            entry: <T as Enterable>::fetch_from_fs(entry_file_path)?,
        })
    }

    fn store_to_fs(self, path: &std::path::Path) -> Result<(), Error> {
        // id
        let id_file_path = path.join("id");
        let mut id_file = std::fs::File::create(&id_file_path)?;
        write!(id_file, "{}", self.id)?;

        // author
        let author_file_path = path.join("author");
        let mut author_file = std::fs::File::create(&author_file_path)?;
        write!(author_file, "{}", self.author)?;

        // created
        let creation_time_file_path = path.join("creation_time");
        let mut creation_time_file = std::fs::File::create(&creation_time_file_path)?;
        write!(creation_time_file, "{}", self.creation_time.to_rfc3339())?;

        // entry
        // TODO: probably need to figure out better naming than "entry"
        // TODO: fix this pathbuf nonsense woooooof
        let entry_file_path_buf = path.join("entry");
        let entry_file_path = entry_file_path_buf.as_path();
        self.entry.store_to_fs(entry_file_path)?;
        Ok(())
    }
}

mod test {

    use super::*;
    impl Enterable for String {
        fn fetch_from_fs(path: &std::path::Path) -> Result<Self, Error> {
            let mut file = std::fs::File::open(&path)?;
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;
            Ok(buffer)
        }
        fn store_to_fs(self, path: &std::path::Path) -> Result<(), Error> {
            let mut file = std::fs::File::create(&path)?;
            write!(file, "{}", self)?;
            Ok(())
        }
    }

    impl Enterable for Vec<String> {
        fn fetch_from_fs(path: &std::path::Path) -> Result<Self, Error> {
            todo!("implement this");
        }
        fn store_to_fs(self, path: &std::path::Path) -> Result<(), Error> {
            todo!("implement this");
        }
    }

    #[test]
    fn store_load_string_entry() {
        let test_path = std::path::Path::new("./test/entry/0000");
        let entry = Entry::<String> {
            id: String::from("87fa3146b90db61c4ea0de182798a0e5"),
            author: String::from("test"),
            creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-22T21:54:42-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            entry: String::from("entry string"),
        };
        entry.clone().store_to_fs(test_path).unwrap();

        let fs_entry = Entry::<String>::fetch_from_fs(test_path).unwrap();

        assert_eq!(entry, fs_entry);
    }

    #[test]
    fn store_load_vec_string_entry() {
        let test_path = std::path::Path::new("./test/entry/0001");
        let entry = Entry::<Vec<String>> {
            id: String::from("87fa3146b90db61c4ea0de182798a0e5"),
            author: String::from("test"),
            creation_time: chrono::DateTime::parse_from_rfc3339("2025-07-22T21:54:42-06:00")
                .unwrap()
                .with_timezone(&chrono::Local),
            entry: vec![String::from("string 1"), String::from("string 2")],
        };
        entry.clone().store_to_fs(test_path).unwrap();

        let fs_entry = Entry::<Vec<String>>::fetch_from_fs(test_path).unwrap();

        assert_eq!(entry, fs_entry);
    }
}
