use serde::de::DeserializeOwned;

pub struct HTTPClient {
    pub authorization: String
}

 impl HTTPClient {

    #[warn(dead_code)]
    pub async fn get_async<T : DeserializeOwned>(&self) -> Result<T, Box<dyn std::error::Error>> {
        let resp = reqwest::get("https://httpbin.org/ip")
            .await?
            .json::<T>()
            .await?;
        Ok(resp)
    }


     pub fn get<T : DeserializeOwned>(&self) -> Result<T, Box<dyn std::error::Error>> {
         let resp = reqwest::blocking::get("https://httpbin.org/ip")?
             .json::<T>()?;
         Ok(resp)
     }



}




