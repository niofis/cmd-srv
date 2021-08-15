use futures::stream::StreamExt;
use paho_mqtt as mqtt;
use std::error::Error;
use std::fs;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

fn shutdown() {
    println!("shutting down");
    let output = Command::new("sudo")
        .arg("/usr/sbin/shutdown")
        .arg("-hP")
        .arg("now")
        .output()
        .expect("failed to shutdown");
    let output_string = String::from_utf8_lossy(&output.stdout);
    let error_string = String::from_utf8_lossy(&output.stderr);
    println!("{}{}", output_string, error_string);
}

fn reboot() {
    println!("rebooting");
    let output = Command::new("sudo")
        .arg("/usr/sbin/reboot")
        .output()
        .expect("failed to reboot");
    let output_string = String::from_utf8_lossy(&output.stdout);
    let error_string = String::from_utf8_lossy(&output.stderr);
    println!("{}{}", output_string, error_string);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let hostname = fs::read_to_string("/etc/hostname")
        .expect("Couldn't read the hostname file")
        .replace("\n", "");
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri("tcp://rpi2.local:1883")
        .client_id(&hostname)
        .finalize();
    let mut cli = mqtt::AsyncClient::new(create_opts)?;
    let mut strm = cli.get_stream(25);
    let lwt = mqtt::Message::new("test", "Async subscriber lost connection", mqtt::QOS_1);
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(30))
        .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
        .clean_session(false)
        .will_message(lwt)
        .finalize();

    println!("Connecting to the MQTT server...");
    cli.connect(conn_opts).await?;

    let commands_topic = format!("{}/commands", &hostname);
    println!("Subscribing to topic: {}", commands_topic);

    cli.subscribe(commands_topic, 1).await?;
    println!("Waiting for messages...");

    while let Some(msg_opt) = strm.next().await {
        if let Some(msg) = msg_opt {
            println!("received: {}", msg.payload_str());
            match msg.topic() {
                topic if topic.ends_with("/commands") => match msg.payload_str().as_ref() {
                    "shutdown" => shutdown(),
                    "reboot" => reboot(),
                    _ => {}
                },
                _ => {}
            }
        } else {
            println!("Lost connection. Attempting reconnect.");
            while let Err(err) = cli.reconnect().await {
                println!("Error reconnecting: {}", err);
                sleep(Duration::from_millis(1000)).await;
            }
        }
    }

    Ok(())
}
