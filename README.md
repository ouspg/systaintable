# syslvltaint

System level taint analysis

## Tooling quick Start guide

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

* The Lense output is saved to outputssh.txt
