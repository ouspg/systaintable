# syslvltaint

System level taint analysis

## Tooling quick Start guide

* Install/clone Semantti Lense https://github.com/semantti/lense
* Install/clone Semantti Prepare https://github.com/semantti/prepare
* Build Semantti Lense

```console
cd lense
make
```

* Build Semantti Prepare

```console
cd ../prepare
make
make install
```

* Go to the lense folder

```console
cd ../lense
```

* Download OpenSSH test corpus https://github.com/logpai/loghub/blob/master/OpenSSH/OpenSSH_2k.log and following command run

```console
tail -n 10000 OpenSSH_2k.log | lense -f json -P -d dictionaries/system/openssh.dict &>> outputssh.txt
```

* The log file Lense output is saved to outputssh.txt