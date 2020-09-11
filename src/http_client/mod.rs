use serde::de::DeserializeOwned;

pub struct HTTPClient {
    pub authorization: String
}

 impl HTTPClient {

     pub fn get<T : DeserializeOwned>(&self) -> Result<T, Box<dyn std::error::Error>> {
         let resp = reqwest::blocking::get("https://httpbin.org/ip")?
             .json::<T>()?;
         Ok(resp)
     }



}




