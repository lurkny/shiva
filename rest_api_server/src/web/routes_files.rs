
use std::collections::HashMap;
use axum::body::Bytes;
use axum::extract::{Multipart, Path};
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use clap::{Parser, ValueEnum};
use serde::Serialize;
use shiva::core::{Document, TransformerTrait};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct DownloadFile {
    file_name: String,
    file_data: (Bytes, HashMap<String, Bytes>),
}

impl IntoResponse for DownloadFile {
    fn into_response(self) -> Response {
        use axum::http::HeaderValue;

        let mut res = self.file_data.0.into_response();
        res.headers_mut().insert(
            "Content-Disposition",
            HeaderValue::from_bytes(self.file_name.as_bytes()).unwrap(),
        );

        res
    }
}

#[derive(Debug, Clone, Parser, ValueEnum)]
enum Format {
    Markdown,
    Html,
    Text,
    Pdf,
}

#[derive(Debug, Clone, Serialize)]
struct InputFileInfo {
    upload_file_name: String,
    upload_file_extension: String,
    upload_file_data: Bytes,
}


pub async fn handler_convert_file(
    Path(output_format): Path<String>,
    multipart: Multipart,
) -> impl IntoResponse {
    println!("-->> {:<12} - handler_convert_file - output_extension_{output_format}", "HANDLER");

    let data_uploaded_file = upload_file(multipart).await.unwrap();

    let build_response_file = convert_file(
        data_uploaded_file.upload_file_name,
        data_uploaded_file.upload_file_extension,
        data_uploaded_file.upload_file_data,
        output_format,
    ).await.unwrap();

    build_response_file
}


async fn convert_file(
    file_name: String,
    file_extension: String,
    input_file_data_bytes: Bytes,
    output_format: String,
) -> Result<DownloadFile> {

    println!("{}", file_name);
    println!("{}", file_extension);
    println!("{}", output_format);
    println!("{:?}", input_file_data_bytes);

    let document = match file_extension.as_str() {

        "md" => Document::from(
            shiva::markdown::Transformer::parse(&input_file_data_bytes, &HashMap::new()).unwrap()
        ),
        "html" | "htm" => Document::from(
            shiva::html::Transformer::parse(&input_file_data_bytes, &HashMap::new()).unwrap()
        ),
        "txt" => Document::from(
            shiva::text::Transformer::parse(&input_file_data_bytes, &HashMap::new()).unwrap()
        ),
        "pdf" => Document::from(
            shiva::pdf::Transformer::parse(&input_file_data_bytes, &HashMap::new()).unwrap()
        ),
        "json" => Document::from(
            shiva::json::Transformer::parse(&input_file_data_bytes, &HashMap::new()).unwrap()
        ),
        _ => return Err(Error::FailParseDocument),
    };

    println!("документ создан");


    let output_bytes = match output_format.as_str() {
        "md" => shiva::markdown::Transformer::generate(&document).unwrap(),
        "html" | "htm" => shiva::html::Transformer::generate(&document).unwrap(),
        "txt" => shiva::text::Transformer::generate(&document).unwrap(),
        "pdf" => shiva::pdf::Transformer::generate(&document).unwrap(),
        "json" => shiva::json::Transformer::generate(&document).unwrap(),
        _ => return Err(Error::FailConvertFile),
    };

    println!("документ конвертирован");

    Ok(DownloadFile {
        file_name,
        file_data: output_bytes,
    })
}

async fn upload_file(mut multipart: Multipart) -> Result<InputFileInfo> {

    println!("запуск upload_file");

    let mut file_name = None;
    let mut file_extension = None;
    let mut file_data = Bytes::new();

    while let Some(field) = multipart.next_field().await.unwrap() {

        println!("извлекаем данные");

        let name = field.name().unwrap_or("").to_string();
        let filename = field.file_name().unwrap_or("").to_string();

        if name == "file" {
            file_name = Some(filename.clone());

            file_name = filename
                .split(".")
                .next()
                .map(|upload_name|upload_name.to_lowercase())
                .filter(|upload_name| !upload_name.trim().is_empty())
                .map(String::from);

            println!("file_name: {:?}", file_name);

            file_extension = filename
                .split(".")
                .last()
                .map(|ext| ext.to_lowercase())
                .filter(|ext| !ext.trim().is_empty())
                .map(String::from);

            println!("file_extension: {:?}", file_extension);

            if let Some(ref ext) = file_extension {

                println!("запуск supported_format");

                if supported_format(ext) {

                    println!("извлекаем Data");

                    file_data = field.bytes().await.unwrap();
                } else {
                    return Err(Error::FailBytes)
                }
            } else {
                return Err(Error::UnsupportedFormat)
            }
        }
    }
    let file_name = file_name.unwrap_or("Shiva_convert".to_string());
    let file_extension = file_extension.ok_or("File extension not found").unwrap();
    let file_data = file_data;

    Ok(InputFileInfo {
        upload_file_name: file_name,
        upload_file_extension: file_extension,
        upload_file_data: file_data,
    })
}

fn supported_format(file_extension: &str) -> bool {

    match file_extension {
        "md" | "html" | "htm" | "txt" | "pdf" | "json" => true,

        _ => false,
    }
}
