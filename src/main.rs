mod h5;

use std::path::PathBuf;

fn main() {
    let fname = PathBuf::from("data/977695_00069191.hdf");
    let file = h5::H5File::open(fname).unwrap();
    let entry = file.load(&h5::H5Path::from("entry".to_string())).unwrap();
    dbg!(&entry);
}
