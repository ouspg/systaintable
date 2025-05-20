# Lense quick start guide

* Install Semantti Lense https://github.com/semantti/lense

```console
git clone git@github.com:semantti/lense.git
```

* Install Semantti Prepare https://github.com/semantti/prepare

```console
git clone git@github.com:semantti/prepare.git
```

* Build Semantti Lense

```console
cd lense
make
make install
cd ..
```

* Build Semantti Prepare

```console
cd prepare
make
make install
cd ..
```

* Go to the lense folder and download OpenSSH test corpus https://github.com/logpai/loghub/blob/master/OpenSSH/OpenSSH_2k.log 

```console
cd lense
curl -O https://raw.githubusercontent.com/logpai/loghub/refs/heads/master/OpenSSH/OpenSSH_2k.log
```

* Run following command

```console
tail -n 10 OpenSSH_2k.log | bin/lense --statistics -f json -P -d dictionaries/system/openssh.dict > outputssh.txt
```

* Lense output is saved to outputssh.txt. Example output below:

```console
Dec 10 11:04:41 LabSZ sshd[25541]: pam_unix(sshd:auth): authentication failure; logname= uid=0 euid=0 tty=ssh ruser= rhost=183.62.140.253 user=root
Dec 10 11:04:42 LabSZ sshd[25539]: Invalid user user from 103.99.0.122
Dec 10 11:04:42 LabSZ sshd[25539]: input_userauth_request: invalid user user [preauth]
Dec 10 11:04:42 LabSZ sshd[25539]: pam_unix(sshd:auth): check pass; user unknown
Dec 10 11:04:42 LabSZ sshd[25539]: pam_unix(sshd:auth): authentication failure; logname= uid=0 euid=0 tty=ssh ruser= rhost=103.99.0.122
Dec 10 11:04:43 LabSZ sshd[25544]: pam_unix(sshd:auth): authentication failure; logname= uid=0 euid=0 tty=ssh ruser= rhost=183.62.140.253 user=root
dictionaries/system/openssh.dict + (null): 4/10 matched with 222 rules, 40.00% coverage, 0.00s elapsed, 3336 rows/s

$ cat outputssh.txt 
["sshd.received_disconnect+1", {"time": 1702206281}, {"host.logging": "LabSZ"}, {"process": "_proc.auth"}, {"exit": "_proc.auth"}, {"pid": 25537}, {"host": "_address"}, {"address": "183.62.140.253"}, {"exit": "_address"}, {"nat": 11}, {"rcvd_disconnect_reason": "_dcr"}, {"connection_state": "_cs"}, {"exit": "_cs"}, {"exit": "_dcr"}]
["sshd.failed-password", {"time": 1702206283}, {"host.logging": "LabSZ"}, {"process": "_proc.auth"}, {"exit": "_proc.auth"}, {"pid": 25541}, {"user": "root"}, {"host": "_address"}, {"address": "183.62.140.253"}, {"exit": "_address"}, {"port": 36300}, {"sshd.protocol_version": "ssh2"}]
["sshd.received_disconnect+1", {"time": 1702206283}, {"host.logging": "LabSZ"}, {"process": "_proc.auth"}, {"exit": "_proc.auth"}, {"pid": 25541}, {"host": "_address"}, {"address": "183.62.140.253"}, {"exit": "_address"}, {"nat": 11}, {"rcvd_disconnect_reason": "_dcr"}, {"connection_state": "_cs"}, {"exit": "_cs"}, {"exit": "_dcr"}]
["sshd.failed_for_invalid_user+0", {"time": 1702206285}, {"host.logging": "LabSZ"}, {"process": "_proc.auth"}, {"exit": "_proc.auth"}, {"pid": 25539}, {"sshd.failed.reason": "password"}, {"user": "user"}, {"host": "_address"}, {"address": "103.99.0.122"}, {"exit": "_address"}, {"port": 52683}, {"sshd.protocol_version": "ssh2"}]
```

## Creating Lense Rules 

Using Lense starts by ...

### Rules and Dictionaries

Lense uses Rules to compress logfiles by matching rules given by user to Lense against the data from logfiles. As part of the compression Lense removes all the unnecessary boiler plate from logsfiles and preserves information that is determined by Rules.

A collection of Lense rules is known as Dictionary. Dictionaries can contain unlimited amount of Lense rules. You can give one or more dictionaries to Lense as a parameter. Some sample dictionaries for processing typical logfiles can be found from Lense repository under dictionaries folder. You can give dictionaries to lense with -d option for example running.

```console
tail -n 10 OpenSSH_2k.log | bin/lense --statistics -f json -P -d dictionaries/system/openssh.dict > outputssh.txt
```

### Creating Lense Rules and dictionaries.

Recommended way to create Lense Rules and dictionaries for your use case is either manually by going through you log material and creating Rules that you need one by one or Semantti Finite-state Loom which is an automated way for creating Rules that match your needs.

### Using Semantti Finite-state Loom

Semantti Finite-state Loom is an automated way to 