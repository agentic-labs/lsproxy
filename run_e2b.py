from dotenv import load_dotenv
load_dotenv()
from e2b import Sandbox  
from time import sleep           
import json      
import random    
import time                                                                                                                                                                                                                                            
                                                                                                                                                                                                                                                                                             
# Create sandbox                                                                                                                                                                                                                                                                         
sbx = Sandbox("5mop5o8nj7nqir0v5fv9", timeout=600 * 2) 

sbx.commands.run("cd /mnt/ && git clone https://github.com/apache/kafka workspace")
command = sbx.commands.run('PATH="/app/venv/bin:/usr/local/cargo/bin:$PATH" RUSTUP_HOME=/usr/local/rustup /lsproxy', background=True, timeout=None, request_timeout=None)
sleep(240)
# tail the logs in /usr/src/app/jdtls_workspace/.logs
# list_files_in_workspace_command = sbx.files.list("/")
# print(list_files_in_workspace_command)
# breakpoint()
# tail_logs_command = sbx.commands.run("tail -f /usr/src/app/jdtls_workspace/.logs", background=True, timeout=None, request_timeout=None)


list_files_lsproxy_command = sbx.commands.run("curl -H 'Accept: application/json' http://localhost:4444/v1/workspace/list-files",  timeout=180, request_timeout=180)
files = json.loads(list_files_lsproxy_command.stdout)
# pick a random file
random_file = random.choice(files)
# url encode the file path
from urllib.parse import quote
encoded_file_path = quote(random_file)


symbols_in_file = json.loads(sbx.commands.run(f"curl -H 'Accept: application/json' 'http://localhost:4444/v1/symbol/definitions-in-file?file_path={random_file}'").stdout)
random_symbol = random.choice(symbols_in_file)
print(random_symbol)
# get references to the symbol
# Prepare request payload for finding references
references_request = {
    "identifier_position": random_symbol["identifier_position"],
    "include_code_context_lines": 3,
    "include_raw_response": False
}

# Call find-references endpoint
#do it a few times and compare the results
references_responses = []
for i in range(10):
    if i == 0:
        breakpoint()
    start_time = time.time()
    try:
        references_response = json.loads(sbx.commands.run(
            f"""curl -X POST -H 'Content-Type: application/json' -H 'Accept: application/json' \
            http://localhost:4444/v1/symbol/find-references \
            -d '{json.dumps(references_request)}'""", timeout=180, request_timeout=180).stdout)
        references_responses.append(references_response)
        print(f"Time taken: {time.time() - start_time} seconds")
        if i == 0:
            breakpoint()
    except Exception as e:
        print(f"Error: {e}")
        print(f"Time taken: {time.time() - start_time} seconds")
        breakpoint()

# check if the results are the same
if references_responses.count(references_responses[0]) == len(references_responses) and len(references_responses[0]["references"]) != 0:
    print("The references results are the same")
else:
    print("The references results are different")# pick random reference and get definition
breakpoint()
random_reference = random.choice(references_responses[0]["references"])

definition_responses = []
for _ in range(10):
    definition_request = {
        "position": random_symbol["identifier_position"],
        "include_raw_response": False,
        "include_source_code": False
    }
    definition_response = json.loads(sbx.commands.run(
        f"""curl -X POST -H 'Content-Type: application/json' -H 'Accept: application/json' \
        http://localhost:4444/v1/symbol/find-definition \
        -d '{json.dumps(definition_request)}'""").stdout)
    definition_responses.append(definition_response)

# check if the results are the same
if definition_responses.count(definition_responses[0]) == len(definition_responses) and len(definition_responses[0]["definitions"]) != 0:
    print("The definition results are the same")
else:
    print("The definition results are different")
    breakpoint()

