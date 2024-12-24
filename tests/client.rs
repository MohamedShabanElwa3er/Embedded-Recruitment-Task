/*!
 * \file client.rs
 * \author Mohamed Shaban Waaer
 * \date 2024-12-24
 * \brief Implements a TCP client for communicating with a server.
 *
 * This module defines a `Client` struct that can connect to a server via TCP, send messages,
 * receive messages with retries, and disconnect. It uses the `prost` crate for message encoding
 * and decoding, and includes basic error handling.
 *
 */

 use embedded_recruitment_task::message::{client_message, ServerMessage};
 use log::{error, info};
 use prost::Message;
 use std::io::{self, Read, Write};
 use std::{
     net::{SocketAddr, TcpStream, ToSocketAddrs},
     time::Duration,
 };
 use std::thread;
 
 /// \brief Represents a TCP client that communicates with a server.
 pub struct Client {
     ip: String,
     port: u32,
     timeout: Duration,
     stream: Option<TcpStream>,
 }
 
 impl Client {
     /*
      * \brief Creates a new instance of the Client.
      *
      * This function initializes the client with the specified IP address, port, and timeout duration.
      *
      * \param ip The IP address of the server.
      * \param port The port number of the server.
      * \param timeout_ms The timeout duration in milliseconds for socket operations.
      * \return A new `Client` instance.
      */
     pub fn new(ip: &str, port: u32, timeout_ms: u64) -> Self {
         Client {
             ip: ip.to_string(),
             port,
             timeout: Duration::from_millis(timeout_ms),
             stream: None,
         }
     }
 
     /*
      * \brief Connects the client to the server.
      *
      * This function resolves the address and attempts to establish a TCP connection
      * with the server at the specified IP and port. If successful, the connection is
      * saved in the `stream` field.
      *
      * \return A result indicating success or failure of the connection attempt.
      */
     pub fn connect(&mut self) -> io::Result<()> {
         println!("Connecting to {}:{}", self.ip, self.port);
 
         // Resolve the address
         let address = format!("{}:{}", self.ip, self.port);
         let socket_addrs: Vec<SocketAddr> = address.to_socket_addrs()?.collect();
 
         if socket_addrs.is_empty() {
             return Err(io::Error::new(
                 io::ErrorKind::InvalidInput,
                 "Invalid IP or port",
             ));
         }
         let stream = TcpStream::connect(format!("localhost:{}", self.port));
         self.stream = Some(stream?);
         println!("Connected to the server!");
         Ok(())
     }
 
     /*
      * \brief Disconnects the client from the server.
      *
      * This function shuts down the TCP connection, ensuring that both send and receive
      * channels are closed properly.
      *
      * \return A result indicating success or failure of the disconnection process.
      */
     pub fn disconnect(&mut self) -> io::Result<()> {
         if let Some(stream) = self.stream.take() {
             stream.shutdown(std::net::Shutdown::Both)?;
         }
 
         println!("Disconnected from the server!");
         Ok(())
     }
 
     /*
      * \brief Sends a message to the server.
      *
      * This function encodes the provided `client_message::Message` into a byte buffer and
      * sends it to the server via the established TCP connection.
      *
      * \param message The message to send to the server.
      * \return A result indicating success or failure of the sending process.
      */
     pub fn send(&mut self, message: client_message::Message) -> io::Result<()> {
         if let Some(ref mut stream) = self.stream {
             // Encode the message to a buffer
             let mut buffer = Vec::new();
             message.encode(&mut buffer);
 
             // Send the buffer to the server
             stream.write_all(&buffer)?;
             stream.flush()?;
 
             println!("Sent message: {:?}", message);
             Ok(())
         } else {
             Err(io::Error::new(
                 io::ErrorKind::NotConnected,
                 "No active connection",
             ))
         }
     }
 
     /*
      * \brief Receives a message from the server with retries.
      *
      * This function attempts to read a message from the server with the specified number of retries.
      * If the read operation fails, the function will retry the specified number of times before
      * returning an error.
      *
      * \param retries The number of retries in case of failure.
      * \return The received `ServerMessage` if successful.
      * \throws io::Error if no message is received after retries or other errors occur.
      */
     pub fn receive_with_retry(&mut self, retries: u32) -> io::Result<ServerMessage> {
         if let Some(ref mut stream) = self.stream {
             let timeout = Duration::from_secs(30);
             stream.set_read_timeout(Some(timeout))?;
 
             let mut buffer = vec![0u8; 4096];
 
             for _ in 0..retries {
                 match stream.read(&mut buffer) {
                     Ok(bytes_read) => {
                         if bytes_read == 0 {
                             info!("Server disconnected.");
                             return Err(io::Error::new(
                                 io::ErrorKind::ConnectionAborted,
                                 "Server disconnected",
                             ));
                         }
 
                         info!("Received {} bytes from the server", bytes_read);
                         return ServerMessage::decode(&buffer[..bytes_read]).map_err(|e| {
                             io::Error::new(
                                 io::ErrorKind::InvalidData,
                                 format!("Failed to decode ServerMessage: {}", e),
                             )
                         });
                     },
                     Err(e) => {
                         error!("Error reading from server: {}", e);
                         thread::sleep(Duration::from_secs(2));  // Retry delay
                     }
                 }
             }
 
             Err(io::Error::new(io::ErrorKind::TimedOut, "Failed to receive message after retries"))
         } else {
             error!("No active connection");
             Err(io::Error::new(
                 io::ErrorKind::NotConnected,
                 "No active connection",
             ))
         }
     }
 }
 