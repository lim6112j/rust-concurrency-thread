use failure::Fail;
use futures::executor::block_on;
use futures01::{future, Future};
use reqwest::r#async::Client as AsyncClient;
use reqwest::{Client, Url};
use std::{thread, env};
use std::thread::ThreadId;
use std::time::{Duration, SystemTime};
use tokio_threadpool::{Builder, SpawnHandle};

pub type ResultWithError<T> = std::result::Result<T, ErrorWrapper>;

#[derive(Fail, Debug)]
pub enum ErrorWrapper {
    #[fail(display = "http request error: {:?}", error)]
    HttpRequestError { error: reqwest::Error },
}

impl From<reqwest::Error> for ErrorWrapper {
    fn from(error: reqwest::Error) -> Self {
        ErrorWrapper::HttpRequestError { error }
    }
}
const URL: &str = "http://localhost:8000";

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut target = 1;
    if args.len() > 1 {
    dbg!(&args[1]);
    target = args[1].parse().unwrap();
    }
    match target {
        1 => request_with_multi_thread(),
        2 => block_on(request_with_async_await()), // this is not a parallel request example
        _ => request_with_main_thread(),
    }
}

fn request_with_main_thread() {
    output1("START SINGLE THREAD");
    let start_at = SystemTime::now();

    let client = Client::new();
    (0..100)
        .collect::<Vec<i32>>()
        .iter()
        .for_each(|i| match send_request(&client, *i) {
            Ok((thread_id, text)) => output2(thread_id, text.as_str()),
            Err(error) => output1(format!("{:?}", error).as_str()),
        });

    let spent_time = start_at.elapsed().unwrap_or(Duration::new(0, 0));
    output1(format!("END: {}", spent_time.as_millis()).as_str())
}

fn send_request(client: &Client, idx: i32) -> ResultWithError<(ThreadId, String)> {

    let url = format!("{}/{}",URL, idx);
    let mut response = client
        .get(Url::parse(&url).unwrap())
        .send()?;
    Ok((thread::current().id(), response.text()?))
}

fn request_with_multi_thread() {
    output1("START MULTI THREAD");
    let start_at = SystemTime::now();

    let client = AsyncClient::new();
    let thread_pool_size = 10;
    let thread_pool = Builder::new().pool_size(thread_pool_size).build();

    let mut handles = Vec::<SpawnHandle<(ThreadId, String), ErrorWrapper>>::new();
    let mut idx = 0;
    while handles.iter().count() <= 500 {
        let cloned_client = client.clone();
        handles.push(thread_pool.spawn_handle(future::lazy(move || {
            send_request_for_future(&cloned_client, idx)
        })));
        idx += 1;
    }

    handles.iter_mut().for_each(|handle| match handle.wait() {
        Ok((thread_id, text)) => output2(thread_id, text.as_str()),
        Err(error) => output1(format!("{:?}", error).as_str()),
    });

    thread_pool.shutdown_now();

    let spent_time = start_at.elapsed().unwrap_or(Duration::new(0, 0));
    output1(format!("END: {}", spent_time.as_millis()).as_str())
}

fn send_request_for_future(
    client: &AsyncClient, idx : i32
) -> impl Future<Item = (ThreadId, String), Error = ErrorWrapper> {
    let url = format!("{}/{}",URL, idx);
    client
        .get(Url::parse(&url).unwrap())
        .send()
        .and_then(|mut response| response.text())
        .map(|text| (thread::current().id(), text))
        .from_err()
}

async fn request_with_async_await() {
    output1("START ASYNC AWAIT");
    let start_at = SystemTime::now();

    let client = Client::new();
    let future1 = send_request_async(&client);
    let future2 = send_request_async(&client);
    match future1.await {
        Ok((thread_id, text)) => output2(thread_id, text.as_str()),
        Err(error) => output1(format!("{:?}", error).as_str()),
    };
    match future2.await {
        Ok((thread_id, text)) => output2(thread_id, text.as_str()),
        Err(error) => output1(format!("{:?}", error).as_str()),
    };

    let spent_time = start_at.elapsed().unwrap_or(Duration::new(0, 0));
    output1(format!("END: {}", spent_time.as_millis()).as_str())
}

async fn send_request_async(client: &Client) -> ResultWithError<(ThreadId, String)> {
    let mut response = client
        .get(Url::parse(URL).unwrap())
        .send()?;
    Ok((thread::current().id(), response.text()?))
}

fn output2(thread_id: ThreadId, text: &str) {
    println!("[{:?}] => {}", thread_id, text);
}

fn output1(text: &str) {
    output2(thread::current().id(), text);
}
