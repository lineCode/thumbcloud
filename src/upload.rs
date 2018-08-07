use actix_web;
use actix_web::error::Error;
use actix_web::http::header::DispositionParam::{Ext, Filename};
use actix_web::*;
use futures::future;
use futures::{Future, Stream};
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

use config::Config;

// This function is a secure version of the join method for PathBuf. The standart join method can
// allow path tranversal, this function doesn't.
fn secure_join<P: AsRef<Path>>(first: PathBuf, second: P) -> Result<PathBuf, io::Error> {
    let mut result = first.clone();
    result = result.join(second);
    result = result.canonicalize()?;

    // Check if first is still a parent of result
    if result.starts_with(first) {
        Ok(result)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Paths are not securely joinable",
        ))
    }
}

fn save_file(
    field: multipart::Field<actix_web::dev::Payload>,
    config: &Config,
) -> Box<Future<Item = i64, Error = Error>> {
    let raw = match field.content_disposition() {
        Some(e) => e.parameters,
        None => {
            return Box::new(future::err(error::ErrorInternalServerError(
                "no valid file",
            )))
        }
    };

    let mut raw_file_name: Vec<u8> = Vec::new();
    let mut file_path = String::new();

    for i in raw.iter() {
        match i {
            Filename(_, _, a) => raw_file_name = a.to_vec(),
            Ext(name, path) => {
                if name == "name" {
                    file_path = path.trim().to_string();
                }
            }
        }
    }

    if raw_file_name.len() == 0 {
        return Box::new(future::err(error::ErrorInternalServerError(
            "no valid file",
        )));
    }

    let file_name = PathBuf::from(match String::from_utf8(raw_file_name.clone()) {
        Ok(n) => n,
        Err(e) => return Box::new(future::err(error::ErrorInternalServerError(e))),
    });

    // Check if the filename is just a filename without a path
    let pure_file_name = match file_name.file_name() {
        Some(name) => name,
        None => return Box::new(future::err(error::ErrorInternalServerError(""))),
    };
    if pure_file_name != file_name {
        return Box::new(future::err(error::ErrorInternalServerError("")));
    }


    println!("Upload: {:?}", file_name);

    let absolute_path = match secure_join(config.path.clone(), file_path) {
        Ok(path) => {
            path.join(file_name.clone())
        }
        Err(e) => return Box::new(future::err(error::ErrorInternalServerError(e))),
    };

    let mut file = match fs::File::create(absolute_path) {
        Ok(file) => file,
        Err(e) => return Box::new(future::err(error::ErrorInternalServerError(e))),
    };

    Box::new(
        field
            .fold(0i64, move |acc, bytes| {
                let rt = file
                    .write_all(bytes.as_ref())
                    .map(|_| acc + bytes.len() as i64)
                    .map_err(|e| {
                        println!("file.write_all failed: {:?}", e);
                        error::MultipartError::Payload(error::PayloadError::Io(e))
                    });
                future::result(rt)
            })
            .map_err(|e| {
                println!("save_file failed, {:?}", e);
                error::ErrorInternalServerError(e)
            }),
    )
}

pub fn handle_multipart_item<'a>(
    item: multipart::MultipartItem<actix_web::dev::Payload>,
    config: &'a Config,
) -> Box<Stream<Item = i64, Error = Error>> {
    let confignew = config.clone();

    match item {
        multipart::MultipartItem::Field(field) => Box::new(save_file(field, &config).into_stream()),
        multipart::MultipartItem::Nested(mp) => Box::new(
            mp.map_err(error::ErrorInternalServerError)
                .map(move |x| handle_multipart_item(x, &confignew))
                .flatten(),
        ),
    }
}