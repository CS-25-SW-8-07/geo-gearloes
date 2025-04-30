>[!NOTE]
> At the moment this repo runs a bevy instance this may not be what we want, it 
> is just for testing that the system is able to run a wasm framework
# Wasm Frontend

This crate is for createing the frontend of our application
it is a wasm binding

# To run
Install `wasm-pack` and `yarn`
```
cargo install wasm-pack
```
```
npm i -g yarn
```

Install wasm as rust target
```
rustup target add wasm32-unknown-unknown
```

To install yarn dependencies 
```
cd webywasm && yarn
```


## Run on linux and mac
```
./build-dev.sh
```

## Run on windows 
```
./please-translate-build-dev.sh-to-powershell.bat
```

