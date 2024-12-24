/*!
 * \file server.rs
 * \author Mohamed Shaban Waaer
 * \date 2024-12-24
 * 
 * \brief This file implements a basic TCP echo server using Rust.
 * 
 * This file contains the implementation of a simple multi-threaded server
 * that listens for client connections, reads messages sent from clients,
 * and echoes the message back to the client. It uses the `prost` library
 * for encoding and decoding messages. The server can handle multiple clients
 * concurrently by spawning new threads to handle each client.
 * 
 * The server listens on a specified address and port, accepts client connections,
 * and spawns new threads to handle client communication. Each client communicates
 * with the server via TCP and sends an `EchoMessage`, which is decoded and sent
 * back to the client.
 * 
 * This file includes two main structures:
 * - `Client`: Represents a single client connection, with methods to handle communication.
 * - `Server`: Represents the server itself, which manages incoming client connections.
 */

 use crate::message::EchoMessage;
 use log::{error, info, warn};
 use prost::Message;
 use std::{
     io::{self, ErrorKind, Read, Write},
     net::{TcpListener, TcpStream},
     sync::{Arc, Mutex},
     thread,
     thread::JoinHandle,
     time::Duration,
 };
 
 /// Represents a client connected to the server.
 struct Client {
     stream: TcpStream,
 }
 
 impl Client {
     /*
      * \brief Constructs a new `Client` instance.
      * 
      * This function initializes a `Client` with the given TCP stream, which represents
      * the connection between the server and the client.
      * 
      * \param stream The TCP stream representing the client's connection.
      * \return A new `Client` instance.
      */
     pub fn new(stream: TcpStream) -> Self {
         Client { stream }
     }
 
     /*
      * \brief Handles communication with the client.
      * 
      * This function continuously reads messages from the client, decodes them, and then
      * sends a response back to the client. If an error occurs during reading or encoding,
      * it returns an error.
      * 
      * \return A result indicating success (`Ok`) or failure (`Err`).
      */
     pub fn handle(&mut self) -> io::Result<()> {
         let mut buffer = [0u8; 4096];
 
         // Keep handling messages as long as the client is connected
         loop {
             let bytes_read = match self.stream.read(&mut buffer) {
                 Ok(0) => {
                     info!("Client disconnected.");
                     return Ok(()); // Client disconnected
                 }
                 Ok(bytes) => bytes,
                 Err(e) => {
                     error!("Error reading from client: {}", e);
                     return Err(e); // Error while reading from client
                 }
             };
 
             // Decode the received message from the buffer
             if let Ok(message) = EchoMessage::decode(&buffer[..bytes_read]) {
                 info!("Received: {}", message.content);
 
                 // Re-encode and send the response
                 let payload = message.encode_to_vec();
                 self.stream.write_all(&payload)?;
                 self.stream.flush()?;
 
                 info!("Sent: {}", message.content);
             } else {
                 error!("Failed to decode message");
             }
         }
     }
 }
 
 /// Represents the echo server.
 pub struct Server {
     max_clients: usize,
     listener: TcpListener,
     is_running: Arc<Mutex<bool>>,
     workers: Vec<JoinHandle<()>>,
 }
 
 impl Server {
     /*
      * \brief Constructs a new `Server` instance.
      * 
      * This function initializes a `Server` with the given address and maximum number
      * of clients. The server will listen for incoming TCP connections and handle them.
      * 
      * \param addr The address the server should bind to.
      * \param max_clients The maximum number of clients the server should handle.
      * \return A result containing the new `Server` instance on success, or an error.
      */
     pub fn new(addr: &str, max_clients: usize) -> io::Result<Self> {
         let listener = TcpListener::bind(addr)?;
         let is_running = Arc::new(Mutex::new(true)); // Ensure server runs until explicitly stopped
         Ok(Server {
             listener,
             is_running,
             workers: Vec::new(),
             max_clients,
         })
     }
 
     /*
      * \brief Runs the server, accepting and handling client connections.
      * 
      * This function continuously accepts incoming client connections and spawns a new
      * thread to handle each client. The server runs until it is explicitly stopped.
      * 
      * \return A result indicating success (`Ok`) or failure (`Err`).
      */
     pub fn run(&mut self) -> io::Result<()> {
         let is_running = self.is_running.clone();
         info!("Server is running on {}", self.listener.local_addr()?);
 
         while *is_running.lock().unwrap() {
             match self.listener.accept() {
                 Ok((stream, addr)) => {
                     info!("New client connected: {}", addr);
 
                     // Create and handle the client in a separate thread
                     let mut client = Client::new(stream);
                     let handle = thread::spawn(move || {
                         if let Err(e) = client.handle() {
                             error!("Error handling client: {}", e);
                         }
                     });
 
                     self.workers.push(handle);
                 }
                 Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                     // Handle non-blocking acceptance, retry after delay
                     thread::sleep(Duration::from_millis(100));
                 }
                 Err(e) => {
                     error!("Error accepting connection: {}", e);
                 }
             }
         }
 
         info!("Server stopped.");
         Ok(())
     }
 
     /*
      * \brief Stops the server by setting the `is_running` flag to `false`.
      * 
      * This function sends a shutdown signal to stop the server from accepting new
      * connections and terminate the running threads.
      */
     pub fn stop(&self) {
         let mut is_running = self.is_running.lock().unwrap();
         if *is_running {
             *is_running = false;
             info!("Shutdown signal sent.");
         } else {
             warn!("Server was already stopped or not running.");
         }
     }
 
     /*
      * \brief Waits for all worker threads to finish.
      * 
      * This function waits for all worker threads handling client connections to
      * complete their tasks before the server fully shuts down.
      */
     pub fn join_workers(&mut self) {
         for worker in self.workers.drain(..) {
             if let Err(e) = worker.join() {
                 error!("Error joining worker thread: {:?}", e);
             }
         }
     }
 }
 