pub mod projects;
pub mod timetrack;

pub struct HTTPClient {
    bearer_token: String,
}

 impl HTTPClient {

     pub fn get<T : DeserializeOwned>(&self) -> Result<T, Box<dyn std::error::Error>> {
         let resp = reqwest::blocking::get("https://httpbin.org/ip")?
             .json::<T>()?;
         Ok(resp)
     }



}
