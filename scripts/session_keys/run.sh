mnemonic_phrase="<mnemonic_phrase>"
validator1_rpc_url="<validator1_rpc_url>"
validator2_rpc_url="<validator2_rpc_url>"
validator3_rpc_url="<validator3_rpc_url>"

curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"audi\", \"${mnemonic_phrase}//1//authority_discovery\", \"0x<authority_discovery public key (hex)>\"]}" ${validator1_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"babe\", \"${mnemonic_phrase}//1//babe\", \"0x<babe public key (hex)>\"]}" ${validator1_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"gran\", \"${mnemonic_phrase}//1//grandpa\", \"0x<grandpa public key (hex)>\"]}" ${validator1_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"imon\", \"${mnemonic_phrase}//1//im_online\", \"0x<im_online public key (hex)>\"]}" ${validator1_rpc_url}

curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"audi\", \"${mnemonic_phrase}//2//authority_discovery\", \"0x<authority_discovery public key (hex)>\"]}" ${validator2_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"babe\", \"${mnemonic_phrase}//2//babe\", \"0x<babe public key (hex)>\"]}" ${validator2_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"gran\", \"${mnemonic_phrase}//2//grandpa\", \"0x<grandpa public key (hex)>\"]}" ${validator2_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"imon\", \"${mnemonic_phrase}//2//im_online\", \"0x<im_online public key (hex)>\"]}" ${validator2_rpc_url}

curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"audi\", \"${mnemonic_phrase}//3//authority_discovery\", \"0x<authority_discovery public key (hex)>\"]}" ${validator3_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"babe\", \"${mnemonic_phrase}//3//babe\", \"0x<babe public key (hex)>\"]}" ${validator3_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"gran\", \"${mnemonic_phrase}//3//grandpa\", \"0x<grandpa public key (hex)>\"]}" ${validator3_rpc_url}
curl -H "Content-Type: application/json" -d "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"author_insertKey\", \"params\":[\"imon\", \"${mnemonic_phrase}//3//im_online\", \"0x<im_online public key (hex)>\"]}" ${validator3_rpc_url}
