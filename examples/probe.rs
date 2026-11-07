use simple_login::smp;

fn main() {
    let client = smp::make_client();
    let url = smp::get_intercept_url(&client);
    let status = smp::do_status(&client);
    println!("intercept = {url:?}");
    println!("status = {status:?}")
}