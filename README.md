# Twitter Sybil System

A toy project to track twitter topic sentiment over time.

https://twitter-sibyl-system.herokuapp.com/

## Build

```shell
cargo build (--release)
```

## Test

```shell
cargo test
```

## Deploy

The service can be deployed automatically via [Heroku](https://www.heroku.com/home).

### Prerequisites

* Install the [Heroku CLI](https://devcenter.heroku.com/articles/heroku-cli#download-and-install) 
* Add Heroku remote git repository
    * `> heroku git:remote -a twitter-sibyl-system`
* Define the [Rust buildpack](https://github.com/emk/heroku-buildpack-rust)
    * `> heroku buildpacks:set emk/rust`

```shell
git push heroku <commit|branch|tag>
```

