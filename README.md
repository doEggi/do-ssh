# do-ssh
Tunnels a ssh-connection over iroh.

## How to use this on the client?
Use the compiled executable as a ProxyCommand for ssh.
```ssh USER@NODEID -p PORT -o "ProxyCommand=do-ssh client %h %p"```
You can use this with password login or key login, it supports everything native ssh supports.
This program is just a proxy that tunnels the ssh-connection over iroh.
Iroh uses direct connections over QUIC, which are End-to-End encrypted, so there is no risk on typing your password or using your keys to login.

Tip: Use the ssh-config file to specify the ProxyCommand:
```
Host EXAMPLE
  Hostname NODEID
  ProxyCommand do-ssh client %h %p
  # IdentityFile ~/.ssh/id_rsa
```
`%h` gets replaced by the hostname provided to ssh.\
`%p` gets replaced by the port proviced to ssh.\
Beware: ssh does not detect USER@HOSTNAME:PORT as single values! You have to provide the port per `-p` to ssh...

## How to use this in the server?
`do-ssh server`.
That's it.
This may generate a persistant SecretKey at the current working directory and starts an iroh endpoint to connect to.
Use the printed NodeId as the hostname for the ssh command and you are good to go.
You can always use `-h` with every subcommand to get a better hang of it.
Also: At default the only allowed port is 22. To allow other ports (or multiple) type `do-ssh server -p 22,2020,2222`.

Tip: Use something like a systemd-service to start the proxy on boot.
Example systemd service file:
```
[Unit]
Description=do-ssh proxy
# This is to ensure we can connect per iroh-relay servers (per NodeId over the internet)
Wants=network-online.target
After=network-online.target

[Service]
# Root is not needed, this can be any user with sufficient privileges
User=root
Group=root
# -n Won't generate a new keyfile, so the server won't ever generate a new NodeId
# -k provides a custom path to the keyfile. There are aliases: -i and -f
ExecStart=do-ssh server -k /root/key.priv -n
# Not needed if a key-file is provided by -k option
WorkingDirectory=/root

[Install]
WantedBy=multi-user.target
```

## Note on generating keys
Every key is a private key. The NodeId is just a public key (derive by its private counterpart). Changing the key changes the NodeId. Only servers need keys, because their NodeIds have to be persistent.
The subcommand `do-ssh generate` or short `do-ssh gen` can be used to just generate private keys. Use `do-ssh -h` or `do-ssh gen -h` for more information.

## Why?
I got a raspberry pi laying around and recently started a project. But the pi often is at a different location and behind a NAT.
To get around this I discovered iroh to create direct connections between devices. So I created this...

## Problems
The delay is notable especially with poor connections.
If the IP of the client or server changes, the connection will most likely rebuild itself automatically, but this may take some seconds.

If a direct connection cannot be established iroh uses its relay servers to transport packages.
Remember, the connection is End-to-End encrypted but I never looked deeply into the source code of iroh.
If you want to be absolutely sure, check out the imo awesome iroh project [here](https://github.com/n0-computer/iroh).

Because the proxy runs on the server itself the only way to log connections (with real addresses) is only possible from the proxy itself (ssh will always show 127.0.0.1 because this is where the TCP-connection comes from).

### TODO: Planned in future:
- Provide compiled releases.
  - Hard because im on macos and cross compiling does only work with certain targets
- Better Code-Documentation.
  - My code mostly documents itself, but docs are important anyways

REMEMBER: This is more a proof of concept. It works (im using it myself) but NodeIds of clients could change (by changing the key) and the implemented iroh-protocol may receive breaking changes in the future.

~doEggi
