use tide::Redirect;
use tide::Request;
use tide::prelude::*;

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::start();
    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());

    app.at("/api/login").serve_file("server_login/www/seventhumbral_redirect.html")?;
    app.at("/login.html").serve_file("server_login/www/login.html")?;
    app.listen("localhost:80").await?;
    Ok(())
}