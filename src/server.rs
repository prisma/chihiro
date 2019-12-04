use crate::SetupOpt;
use std::{io::{BufRead, BufReader}, net::TcpStream};

pub struct Server {
    session: ssh2::Session,
}

impl Server {
    pub fn new(opts: SetupOpt) -> crate::Result<Self> {
        let tcp = TcpStream::connect(&opts.host)?;
        let mut session = ssh2::Session::new()?;

        session.set_tcp_stream(tcp);
        session.handshake()?;

        session.userauth_pubkey_file(
            &opts.user,
            None,
            &opts.private_key,
            opts.passphrase.as_ref().map(|s| s.as_str()),
        )?;

        assert!(session.authenticated());

        Ok(Self { session, })
    }

    pub fn setup(&mut self) -> crate::Result<()> {
        let mut remote = self.session.channel_session()?;
        remote.exec("cd prisma && make")?;

        let buf = BufReader::new(remote);

        for l in buf.lines() {
            println!("{}", l?);
        }

        Ok(())
    }
}
