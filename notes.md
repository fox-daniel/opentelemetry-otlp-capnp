- Tonic uses Channels. What is the equivalent for capnp-rpc?
# From Claude
````
pub struct CapnpConnection {
    _rpc_system_handle: JoinHandle<Result<(), Error>>,
    client: my_service::Client,
}

impl CapnpConnection {
    pub async fn connect(addr: SocketAddr) -> Result<Self, Error> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        
        let (reader, writer) = stream.split();
        let network = Box::new(twoparty::VatNetwork::new(
            reader, writer,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        ));
        
        let mut rpc_system = RpcSystem::new(network, None);
        let client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
        
        let handle = tokio::task::spawn_local(rpc_system);
        
        Ok(Self {
            _rpc_system_handle: handle,
            client,
        })
    }
    
    pub fn client(&self) -> my_service::Client {
        self.client.clone()
    }
}
```
