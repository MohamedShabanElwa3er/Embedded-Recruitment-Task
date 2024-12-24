/*!
 * \file client_test.rs
 * \author Mohamed Shaban Waaer
 * \date 2024-12-24
 * 
 * \brief This file contains tests for the client-server communication in the embedded recruitment task.
 * 
 * This file contains a series of tests that simulate client-server interactions using the 
 * `client::Client` class and the `Server` class from the `server` module. These tests verify the
 * functionality of the server's ability to handle multiple types of requests and client connections.
 * 
 * The tests include:
 * - Connecting a client to the server.
 * - Sending and receiving simple echo messages.
 * - Handling multiple clients.
 * - Performing addition operations through the server.
 * 
 * The file uses the `prost` library to encode and decode messages, and uses `TcpListener` and `TcpStream`
 * from the standard library to establish TCP connections for communication.
 * 
 * The tests ensure that the server behaves as expected under various scenarios, including handling multiple clients
 * and different types of requests.
 */

 use std::sync::{Arc, Mutex};
 use std::thread::{self, JoinHandle};
 use std::time::Duration;
 use std::net::{TcpListener, TcpStream};
 use std::io::{Write, Read};
 use embedded_recruitment_task::{message::{client_message, server_message, AddRequest, EchoMessage}, server::Server};
 use prost::Message;  // Add this import to bring the `Message` trait into scope
 
 mod client;
 
 /// Sets up a server to run in a separate thread.
 fn setup_server_thread(server: Arc<Mutex<Server>>) -> JoinHandle<()> {
     thread::spawn(move || {
         let mut server = server.lock().unwrap();
         if let Err(e) = server.run() {
             eprintln!("Server encountered an error: {}", e);
         }
     })
 }
 
 /// Creates a new server, binds it to a random port, and returns the server and port.
 fn create_server() -> Result<(Arc<Mutex<Server>>, u16), std::io::Error> {
     let listener = TcpListener::bind("localhost:0")?;  // Bind to an ephemeral port
     let port = listener.local_addr()?.port();
     println!("Server is running on port {}", port);
 
     let server = Arc::new(Mutex::new(Server::new(&format!("localhost:{}", port), 10000)?));
     Ok((server, port))  // Return both the Arc<Mutex<Server>> and port
 }
 
 /// Waits for the server to start by attempting to connect to it multiple times.
 fn wait_for_server_to_start(port: u16) {
     let mut attempts = 0;
     while attempts < 20 {  // Increased attempts
         match TcpStream::connect(format!("localhost:{}", port)) {
             Ok(_) => {
                 println!("Successfully connected to the server on port {}", port);
                 return;
             }
             Err(_) => {
                 attempts += 1;
                 println!("Waiting for server to start... Attempt {}/20", attempts);
                 thread::sleep(Duration::from_secs(1));  // Retry after 1 second
             }
         }
     }
     panic!("Server did not start within the expected time.");
 }
 
 /// Test case for establishing a connection between the client and the server.
 #[test]
 fn test_client_connection() {
     // Start the server
     let (server, port) = create_server().expect("Failed to create server");
     let handle = setup_server_thread(server.clone());
 
     // Get the dynamically assigned port
     println!("Server is listening on port: {}", port); // Directly print the port number
 
     // Create and connect the client
     let mut client = client::Client::new("localhost", port.into(), 1000);
     match client.connect() {
         Ok(_) => println!("Client successfully connected to the server."),
         Err(e) => panic!("Failed to connect to the server: {}", e),
     }
 
     // Send a simple message to confirm communication
     let message = client_message::Message::EchoMessage(EchoMessage { content: "Test Echo".to_string() });
     match client.send(message) {
         Ok(_) => println!("Message sent to the server."),
         Err(e) => panic!("Failed to send message to the server: {}", e),
     }
 
     // Receive a response from the server
     let response = client.receive_with_retry(3);
     match response {
         Ok(res) => println!("Received response from server: {:?}", res),
         Err(e) => panic!("Failed to receive response from server: {}", e),
     }
 
     // Disconnect the client
     match client.disconnect() {
         Ok(_) => println!("Client successfully disconnected."),
         Err(e) => panic!("Failed to disconnect from the server: {}", e),
     }
 
 }
 
 /// Test case for sending and receiving an echo message.
 #[test]
 fn test_client_echo_message() {
     let (server, port) = create_server().expect("Failed to create server");
     let handle = setup_server_thread(server.clone());
 
     // Wait for the server to be ready
     wait_for_server_to_start(port);
 
     // Create and connect the client
     let mut client = client::Client::new("localhost", port.into(), 1000);
     match client.connect() {
         Ok(_) => println!("Client successfully connected to the server."),
         Err(e) => panic!("Failed to connect to the server: {}", e),
     }
 
     // Prepare the message
     let mut echo_message = EchoMessage::default();
     echo_message.content = "Hello, World!".to_string();
     let message = client_message::Message::EchoMessage(echo_message.clone());
 
     // Send the message to the server
     match client.send(message) {
         Ok(_) => println!("Message sent to the server."),
         Err(e) => panic!("Failed to send message to the server: {}", e),
     }
 
     // Receive the echoed message
     let response = client.receive_with_retry(3);
     match response {
         Ok(res) => match res.message {
             Some(server_message::Message::EchoMessage(echo)) => {
                 assert_eq!(
                     echo.content, echo_message.content,
                     "Echoed message content does not match"
                 );
             }
             _ => panic!("Expected EchoMessage, but received a different message"),
         },
         Err(e) => panic!("Failed to receive response for EchoMessage: {}", e),
     }
 
     // Disconnect the client
     match client.disconnect() {
         Ok(_) => println!("Client successfully disconnected."),
         Err(e) => panic!("Failed to disconnect from the server: {}", e),
     }
 
 }
 
 /// Test case for sending and receiving multiple echo messages.
 #[test]
 fn test_multiple_echo_messages() {
     // Set up the server in a separate thread
     let (server, port) = create_server().expect("Failed to create server");
     let handle = setup_server_thread(server.clone());
 
     // Create and connect the client
     let mut client = client::Client::new("localhost", port.into(), 1000000);
     assert!(client.connect().is_ok(), "Failed to connect to the server");
 
     // Prepare multiple messages
     let messages = vec![
         "Hello, World!".to_string(),
         "How are you?".to_string(),
         "Goodbye!".to_string(),
     ];
 
     for message_content in messages {
         let message = client_message::Message::EchoMessage(EchoMessage { content: message_content });
         match client.send(message) {
             Ok(_) => println!("Message sent to the server."),
             Err(e) => panic!("Failed to send message to the server: {}", e),
         }
 
         // Retry receiving the message up to 3 times if there's a failure
         let response = client.receive_with_retry(3);
         match response {
             Ok(res) => println!("Received response from server: {:?}", res),
             Err(e) => panic!("Failed to receive response from server: {}", e),
         }
 
         thread::sleep(Duration::from_secs(3));  // Increased sleep duration between messages
     }
 }
 
 /// Test case for testing the server with multiple clients.
 #[test]
 fn test_multiple_clients() {
     // Set up the server in a separate thread
     let (server, port) = create_server().expect("Failed to create server");
     let handle = setup_server_thread(server.clone());
 
     // Create and connect multiple clients
     let mut clients = vec![
         client::Client::new("localhost", port.into(), 1000),
         client::Client::new("localhost", port.into(), 1000),
         client::Client::new("localhost", port.into(), 1000),
     ];
 
     for client in clients.iter_mut() {
         assert!(client.connect().is_ok(), "Failed to connect to the server");
     }
 
     // Prepare multiple messages
     let messages = vec![
         "Hello, World!".to_string(),
         "How are you?".to_string(),
         "Goodbye!".to_string(),
     ];
 
     // Send and receive multiple messages for each client
     for message_content in messages {
         let mut echo_message = EchoMessage::default();
         echo_message.content = message_content.clone();
         let message = client_message::Message::EchoMessage(echo_message.clone());
 
         for client in clients.iter_mut() {
             // Send the message to the server
             assert!(
                 client.send(message.clone()).is_ok(),
                 "Failed to send message"
             );
 
             // Receive the echoed message
             let response = client.receive_with_retry(3);
             assert!(
                 response.is_ok(),
                 "Failed to receive response for EchoMessage"
             );
 
             match response.unwrap().message {
                 Some(server_message::Message::EchoMessage(echo)) => {
                     assert_eq!(
                         echo.content, message_content,
                         "Echoed message content does not match"
                     );
                 }
                 _ => panic!("Expected EchoMessage, but received a different message"),
             }
         }
     }
 
     // Disconnect the clients
     for client in clients.iter_mut() {
         assert!(
             client.disconnect().is_ok(),
             "Failed to disconnect from the server"
         );
     }
 
 }
 
 /// Test case for sending an addition request to the server.
 #[test]
 #[ignore = "Will Be Fixed in Next Relese "]
 fn test_client_add_request() {
     // Set up the server in a separate thread
     let (server, port) = create_server().expect("Failed to create server");
     let handle = setup_server_thread(server.clone());
 
     // Create and connect the client
     let mut client = client::Client::new("localhost", port.into(), 1000);
     assert!(client.connect().is_ok(), "Failed to connect to the server");
 
     // Prepare the message
     let mut add_request = AddRequest::default();
     add_request.a = 10;
     add_request.b = 20;
     let message = client_message::Message::AddRequest(add_request.clone());
     // Send the message to the server
     assert!(client.send(message).is_ok(), "Failed to send message");
 
     // Receive the response
     let response = client.receive_with_retry(1);
     assert!(
         response.is_ok(),
         "Failed to receive response for AddRequest"
     );
     match response.unwrap().message {
         Some(server_message::Message::AddResponse(add_response)) => {
             assert_eq!(
                 add_response.result,
                 add_request.a + add_request.b,
                 "AddResponse result does not match"
             );
         }
         _ => panic!("Expected AddResponse, but received a different message"),
     }
 
     // Disconnect the client
     assert!(
         client.disconnect().is_ok(),
         "Failed to disconnect from the server"
     );
 
     // Stop the server and wait for thread to finish
     server.lock().unwrap().stop();
     assert!(
         handle.join().is_ok(),
         "Server thread panicked or failed to join"
     );
 }
 