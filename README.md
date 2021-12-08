# Janken Contract
CosmWasm Janken contract for Secrete Netork. Janken is the "Rock-Paper-Scissors" in Japanese. The user can bet NFT or Token.

# How to play NFT betting
Taking 2 steps to play with.

### 1st, Making offer
At first, a player “make offer” to the player who owns the NFT the player wants. NFT rarity is different from each NFTs. So to guaranty fairness, object the required win times in total matches. For example, the high rarity nft owner just need to win once in 3 times match.

At the making offer time, a player choose his hands. The hands are hidden against the opponent. Only the `view_key` holder can see it.
```javascript
{
	make_offer: {
		id:                   // offer uniq id
		offeree:              // the player address
		offeror_nft_contract: // the nft contract address
		offeror_nft:          // the nft id
		offeror_code_hash:    // the hash of nft contract
		offeree_nft_contract: // ...
		offeree_nft:          // ...
		offeree_code_hash:    // ... 
		offeror_hands:        // the array of hand numbers, Rock=1, Paper=2, Scissors=3
		offeror_draw_point:   // the offeror win if he get more than this total point, win=1 point, draw=0 point, lose=-1 point
	                              // Ex) if offeror win twice, draw once and lose once, then the total point is "1".
	}
}
```

`offeror_code_hash` and `offeree_code_hashcan` can be get like bellow command.
```sh
secretcli q compute contract-hash $CONTRACT_ADDRESS
```
As example, the snip721 contract code hash in `wasm/snip721.wasm.gz` is `6208b13151f8fba7a474c1b7dfced661a8aa2fb4769049fed8442e4cd1d7f1df`

### 2nd, Accept or Decline offer
The opponent can take 2 actions, a one is “accept”. The other is “decline”. When the opponent accept the offer, the opponent submit his hands. Then, the match is processed in contract.
The winner obtain the looser’s NFT. The NFT transfering is executed in contract, so that both player need to approve Janken contract before match.
```javascript
{
	accept_offer: {
    		id:            // the uniq id of offer, should same as offerd one
    		offeree_hands: // the array of opponent hand numbers
	}
}
```

In the case of decline, just return the id
```javascript
{
	decline_offer: {
		id: // the uniq id of offer, should same as offerd one
	}
}
```

# How to play Token betting
Taking just 1 steps to play with.

A player submit a hand, an amount of betting and a entropy. The entropy is used for random number generation source. To prevent darty play, this random number generation source is accumulated every time on play and never be seen from anyone.
```javascript
{
	bet_token: {
		denom:  // the betting token denom
		amount: // the betting token amount
		hand:   // the player hand
		entropy // the random number generation source
	}
}
```

The matches is processed automatically in the contract. If a player win, a player get “the betting amount - fee” equivalent amount of token. If a player lose, a player lost “the betting amount ” equivalent amount of token. If the match result is draw, a player just pay fee.

# How to generate View Key 
`view_key` is used for seeing own hands in maked offer.
```javascript
{
	generate_viewing_key: {
		entropy: // the random number generation source
		padding: // the optional padding
	}
}
```

# Hands
```
Rock     = 1
Paper    = 2
Scissors = 3
```

# The One Match Point
```
Win  = 1
Draw = 2
Lose = 3
```
