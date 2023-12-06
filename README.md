# qiusuo lemmy

fork from [lemmyNet/lemmy](https://github.com/LemmyNet/lemmy)

## how startup development environment

1. install watch, from binary `cargo binstall cargo-watch` or from source `cargo install cargo-watch`
2. start collaborative service `docker compose -f ./docker-dev/docker-compose.yml up`
3. wait database ready
4. set DATABASE enviroment variable
   1. LEMMY_DATABASE_URL = "postgres://lemmy:postgres@localhost:9031/lemmy"
5. start lemmy service with `cargo watch -x run`

## how deploy server to server manully

1. build lemmy docker image `docker build -t qiusuo/lemmy:0.0.3 -f .\docker\Dockerfile .`
   1. change the version to newtest
2. upload the image to server scp `scp -v .\qs_lemmy.tar root@xxx.xx.xx.xxx:~/` with user `root`
   1. if you don't setup ssh private key, you will be request input password manully
3. output image file with command `docker save --output qs_lemmy.tar qiusuo/lemmy:0.0.3`
4. pass `qs_lemmy.tar` to *lemmy server(another inner network machine)*
5. cd into *lemmy docker config folder*
6. run docker image load command
7. run docker compose up command
