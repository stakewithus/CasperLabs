from pathlib import Path
import os
import logging
import json
import subprocess
from operator import add
from functools import reduce
from test.cl_node.client_parser import parse_show_blocks, parse_show_deploys, parse
from test.cl_node.common import MAX_PAYMENT_COST, resources_path


class CLIErrorExit(Exception):
    def __init__(self, cp, output):
        self.cp = cp
        self.output = output

    def __str__(self):
        return f"{self.cp}: {self.output}"


class CLI:

    _MAX_PAYMENT_JSON = json.dumps(
        [{"name": "amount", "value": {"u512": MAX_PAYMENT_COST}}]
    )

    def __init__(self, node, cli_cmd="casperlabs_client", tls_parameters=None):
        self.node = node
        self.host = (
            os.environ.get("TAG_NAME", None) and node.container_name or "localhost"
        )
        self.port = node.grpc_external_docker_port
        self.internal_port = node.grpc_internal_docker_port
        self.cli_cmd = cli_cmd
        self.tls_parameters = tls_parameters or {}
        self.default_deploy_args = []
        self.resources_directory = resources_path()

    def set_default_deploy_args(self, *args):
        """ Set args that will be appended to subsequent deploy command. """
        self.default_deploy_args = [str(arg) for arg in args]

    def resource(self, file_name):
        return self.resources_directory / file_name

    def expand_args(self, args):
        connection_details = ["--host", f"{self.host}", "--port", f"{self.port}"]
        if self.tls_parameters:
            connection_details += reduce(
                add,
                [[str(p), str(self.tls_parameters[p])] for p in self.tls_parameters],
            )
        string_args = [str(a) for a in list(args)]

        if args and args[0] == "deploy":
            string_args += self.default_deploy_args

        return "--help" in args and string_args or connection_details + string_args

    def parse_output(self, command, binary_output):

        if command in ("make-deploy", "sign-deploy"):
            return binary_output

        output = binary_output.decode("utf-8")

        if command in ("deploy", "send-deploy"):
            return output.split()[2]
            # "Success! Deploy 0d4036bebb95de793b28de452d594531a29f8dc3c5394526094d30723fa5ff65 deployed."

        if command in ("propose",):
            # "Response: Success! Block 47338c65992e7d5062aec2200ad8d7284ae49f6c3e7c37fa7eb46fb6fc8ae3d8 created and added."
            return output.split()[3]

        if command == "show-blocks":
            return parse_show_blocks(output)

        if command == "show-deploys":
            return parse_show_deploys(output)

        if command in ("show-deploy", "show-block", "query-state"):
            return parse(output)

        return output

    def __call__(self, *args):
        command_line = [str(self.cli_cmd)] + self.expand_args(args)
        logging.info(f"EXECUTING []: {command_line}")
        logging.info(f"EXECUTING: {' '.join(command_line)}")
        cp = subprocess.run(
            command_line, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        binary_output = cp.stdout
        if cp.returncode != 0:
            output = binary_output
            try:
                output = binary_output.decode("utf-8")
            except UnicodeDecodeError:
                pass
            raise CLIErrorExit(cp, output)

        return self.parse_output(args[0], binary_output)

    def public_key_path(self, account):
        return account.public_key_path

    def private_key_path(self, account):
        return account.private_key_path

    def format_json_str(self, args: str) -> str:
        return args

    @property
    def payment_json(self) -> str:
        return self.format_json_str(self._MAX_PAYMENT_JSON)


class DockerCLI(CLI):

    _MAX_PAYMENT_JSON = json.dumps(
        [
            {
                "name": "amount",
                "value": {
                    "big_int": {"value": f"{MAX_PAYMENT_COST}", "bit_width": 512}
                },
            }
        ]
    )

    def __init__(self, node, tls_parameters=None):
        super().__init__(node, tls_parameters=tls_parameters)
        self.resources_directory = Path("/data/")

    def __call__(self, *args):
        logging.info(f"EXECUTING []: {args}")
        self.host = self.node.container_name
        command = " ".join(self.expand_args(args))
        logging.info(f"EXECUTING: {command}")
        binary_output = self.node.d_client.invoke_client(
            command, decode_stdout=False, add_host=False
        )
        return self.parse_output(args[0], binary_output)

    def public_key_path(self, account):
        return account.public_key_docker_path

    def private_key_path(self, account):
        return account.private_key_docker_path

    def format_json_str(self, args: str) -> str:
        return f"'{args}'"
