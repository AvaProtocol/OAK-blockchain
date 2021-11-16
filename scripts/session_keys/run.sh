mnemonic_phrase="<mnemonic_phrase>"
validator_rpc_urls=("<validator1_rpc_url>" "<validator2_rpc_url>" "<validator3_rpc_url>")

insert_key(){
  curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"${2}\", \"${mnemonic_phrase}//${1}//${3}\", \"0x${4}\"]}" ${validator_rpc_urls[$1-1]}
}

# Validator1
insert_key 1 'audi' 'authority_discovery' '<authority_discovery public key (hex)>'
insert_key 1 'babe' 'babe' '<babe public key (hex)>'
insert_key 1 'grandpa' 'grandpa' '<grandpa public key (hex)>'
insert_key 1 'imon' 'im_online' '<im_online public key (hex)>'

# Validator2
insert_key 2 'audi' 'authority_discovery' '<authority_discovery public key (hex)>'
insert_key 2 'babe' 'babe' '<babe public key (hex)>'
insert_key 2 'grandpa' 'grandpa' '<grandpa public key (hex)>'
insert_key 2 'imon' 'im_online' '<im_online public key (hex)>'

# Validator3
insert_key 3 'audi' 'authority_discovery' '<authority_discovery public key (hex)>'
insert_key 3 'babe' 'babe' '<babe public key (hex)>'
insert_key 3 'grandpa' 'grandpa' '<grandpa public key (hex)>'
insert_key 3 'imon' 'im_online' '<im_online public key (hex)>'
