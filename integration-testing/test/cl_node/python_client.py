from typing import Optional, Union
import os
import logging
import time
from pathlib import Path  # noqa: F401

from test.cl_node import LoggingMixin
from test.cl_node.common import Contract, MAX_PAYMENT_ABI
from casperlabs_client import CasperLabsClient, ABI, InternalError, extract_common_name


class PythonClient(CasperLabsClient, LoggingMixin):
    def __init__(self, node: "DockerNode"):  # NOQA
        super(PythonClient, self).__init__()
        self.node = node
        self.abi = ABI
        # If $TAG_NAME is set it means we are running in docker, see docker_run_test.sh
        host = (
            os.environ.get("TAG_NAME", None) and self.node.container_name or "localhost"
        )

        certificate_file = None
        node_id = None
        if self.node.config.grpc_encryption:
            certificate_file = self.node.config.tls_certificate_local_path()
            node_id = extract_common_name(certificate_file)

        self.client = CasperLabsClient(
            host=host,
            internal_port=self.node.grpc_internal_docker_port,
            port=self.node.grpc_external_docker_port,
            node_id=node_id,
            certificate_file=certificate_file,
        )
        logging.info(
            f"PythonClient(host={self.client.host}, "
            f"port={self.node.grpc_external_docker_port}, "
            f"internal_port={self.node.grpc_internal_docker_port})"
        )

    @property
    def client_type(self) -> str:
        return "python"

    def deploy(
        self,
        from_address: str = None,
        gas_price: int = 1,
        session_contract: Optional[Union[str, Path]] = None,
        payment_contract: Optional[Union[str, Path]] = Contract.STANDARD_PAYMENT,
        private_key: Optional[str] = None,
        public_key: Optional[str] = None,
        session_args: list = None,
        payment_args: list = MAX_PAYMENT_ABI,
    ) -> str:

        assert session_contract is not None

        public_key = public_key or self.node.test_account.public_key_path
        private_key = private_key or self.node.test_account.private_key_path

        address = from_address or self.node.from_address

        resources_path = self.node.resources_folder
        session_contract_path = str(resources_path / session_contract)
        payment_contract_path = str(resources_path / payment_contract)

        logging.info(
            f"PY_CLIENT.deploy(from_address={address}, gas_price={gas_price}, "
            f"payment_contract={payment_contract_path}, session_contract={session_contract_path}, "
            f"private_key={private_key}, "
            f"public_key={public_key} "
        )

        return self.client.deploy(
            bytes.fromhex(address),
            gas_price,
            payment_contract_path,
            session_contract_path,
            public_key,
            private_key,
            session_args,
            payment_args,
        )

    def propose(self):
        logging.info(f"PY_CLIENT.propose() for {self.client.host}")
        return self.client.propose()

    def query_state(self, block_hash: str, key: str, path: str, key_type: str):
        return self.client.queryState(block_hash, key, path, key_type)

    def show_block(self, block_hash: str):
        return self.client.showBlock(block_hash)

    def show_blocks(self, depth: int):
        return self.client.showBlocks(depth)

    def get_blocks_count(self, depth: int) -> int:
        return len(list(self.show_blocks(depth)))

    def show_deploys(self, block_hash: str):
        return self.client.showDeploys(block_hash)

    def show_deploy(self, deploy_hash: str):
        return self.client.showDeploy(deploy_hash)

    def propose_with_retry(self, max_attempts: int, retry_seconds: int) -> str:
        attempt = 0
        while True:
            try:
                return self.propose()
            except InternalError as ex:
                if attempt < max_attempts:
                    self.logger.debug("Could not propose; retrying later.")
                    attempt += 1
                    time.sleep(retry_seconds)
                else:
                    self.logger.debug("Could not propose; no more retries!")
                    raise ex

    def deploy_and_propose(self, **deploy_kwargs) -> str:
        if "from_address" not in deploy_kwargs:
            deploy_kwargs["from_address"] = self.node.from_address
        self.deploy(**deploy_kwargs)

        propose_output = self.propose()
        block_hash = propose_output.block_hash.hex()

        logging.info(
            f"The block hash: {block_hash} generated for {self.node.container_name}"
        )
        return block_hash

    def deploy_and_propose_with_retry(
        self, max_attempts: int, retry_seconds: int, **deploy_kwargs
    ) -> str:
        self.deploy(**deploy_kwargs)

        block_hash = self.propose_with_retry(max_attempts, retry_seconds)

        logging.info(
            f"The block hash: {block_hash} generated for {self.node.container_name}"
        )
        if block_hash is None:
            raise Exception("No block_hash received from propose_with_retry")

        return block_hash
