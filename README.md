# cosmwasm-gambling
CosmWasm contract of gambling mini game (rock-paper-scissors)

# Note
```sh
scp -P 22 -i ~/.ssh/janken_key.pem -r azureuser@20.102.100.176:/home/azureuser/tak/cosmwasm-gambling/contract.wasm.gz ./
scp -P 22 -i ~/.ssh/janken_key.pem -r ./contract.wasm.gz azureuser@20.121.139.233:/home/azureuser/lab/cosmwasm-gambling/

# deploy
secretcli tx compute store ./contract.wasm.gz --from alice --gas 10000000 -y
secretcli query compute list-code

# init
export INIT=$(jq -n '{}')
export RES=$(secretcli tx compute instantiate 10 "$INIT" --from alice --label janken10 -y --gas 1000000)
export TX=$(echo $RES | jq -r '.txhash')
export RES=$(secretcli q tx $TX)
export CONTRACT=$(echo $RES | jq -r '.logs[0].events[1].attributes[0].value')
# secret1uul3yzm2lgskp3dxpj0zg558hppxk6ptyljer5

# make offer
export MAKEOFFER="{\"make_offer\":{\"id\": 1, \"offeree\": \"$(secretcli keys show bob -a)\", \"offeror_nft_contract\": \"secret1dp972qfjp362m7slfjsvzg6w72ky5reu5he4es\", \"offeror_nft\": \"optional_ID_of_new_token\", \"offeror_code_hash\": \"6208b13151f8fba7a474c1b7dfced661a8aa2fb4769049fed8442e4cd1d7f1df\", \"offeree_nft_contract\": \"secret1dp972qfjp362m7slfjsvzg6w72ky5reu5he4es\", \"offeree_nft\": \"optional_ID_of_new_token2\", \"offeree_code_hash\": \"6208b13151f8fba7a474c1b7dfced661a8aa2fb4769049fed8442e4cd1d7f1df\", \"offeror_hands\": [1, 2, 3], \"offeror_draw_point\": 2}}"
export RES=$(secretcli tx compute execute $CONTRACT "$MAKEOFFER" --from alice -y)
export TX=$(echo $RES | jq -r '.txhash')
secretcli q tx $TX

# accept
secretcli q compute contract-hash $CONTRACT
export ACCEPT="{\"accept_offer\":{\"id\": 1, \"offeree_hands\": [3, 2, 1]}}"
export RES=$(secretcli tx compute execute $CONTRACT "$ACCEPT" --from bob -y)
export TX=$(echo $RES | jq -r '.txhash')
secretcli q tx $TX

# offer
export OFFER=$(jq -n '{"offer":{"id":1}}')
secretcli q compute query $CONTRACT "$OFFER"
```

## Hands
```
Rock     = 1
Paper    = 2
Scissors = 3
```
