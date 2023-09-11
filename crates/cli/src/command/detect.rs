use anyhow::Error;
use fehler::throws;
use trdelnik_client::Detector;

#[throws]
pub async fn detect(module_name: String) {
    let generator = Detector::new(module_name);
    generator.detect().await?;
}
