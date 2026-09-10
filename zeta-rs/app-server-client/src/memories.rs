use crate::AppServerClient;
use crate::ClientError;
use crate::JsonRpcTransport;
use zeta_app_server_protocol::protocol::memory::MemoryAddParams;
use zeta_app_server_protocol::protocol::memory::MemoryDeleteParams;
use zeta_app_server_protocol::protocol::memory::MemoryListParams;
use zeta_app_server_protocol::protocol::memory::MemoryReadParams;
use zeta_app_server_protocol::protocol::memory::MemorySearchParams;
use zeta_app_server_protocol::protocol::registry::ClientMethod;

impl<T: JsonRpcTransport> AppServerClient<T> {
    pub fn add_memory(
        &mut self,
        params: MemoryAddParams,
    ) -> Result<memories::MemoryMutationResult, ClientError> {
        let expected = (params.memory_id.clone(), params.scope.clone());
        let result: memories::MemoryMutationResult = self.call(ClientMethod::MemoryAdd, params)?;
        if result.memory.memory_id != expected.0 || result.memory.scope != expected.1 {
            return Err(ClientError::Protocol(
                "Memory mutation returned another identity".into(),
            ));
        }
        Ok(result)
    }

    pub fn list_memories(
        &mut self,
        params: MemoryListParams,
    ) -> Result<memories::MemoryListPage, ClientError> {
        let scope = params.scope.clone();
        let result: memories::MemoryListPage = self.call(ClientMethod::MemoryList, params)?;
        if result.memories.iter().any(|memory| memory.scope != scope) {
            return Err(ClientError::Protocol(
                "Memory page contains another scope".into(),
            ));
        }
        Ok(result)
    }

    pub fn read_memory(
        &mut self,
        params: MemoryReadParams,
    ) -> Result<memories::Memory, ClientError> {
        let expected = (params.memory_id.clone(), params.scope.clone());
        let result: memories::Memory = self.call(ClientMethod::MemoryRead, params)?;
        if result.memory_id != expected.0 || result.scope != expected.1 {
            return Err(ClientError::Protocol(
                "Memory read returned another identity".into(),
            ));
        }
        Ok(result)
    }

    pub fn search_memories(
        &mut self,
        params: MemorySearchParams,
    ) -> Result<memories::MemorySearchPage, ClientError> {
        let scope = params.scope.clone();
        let result: memories::MemorySearchPage = self.call(ClientMethod::MemorySearch, params)?;
        if result.matches.iter().any(|memory| memory.scope != scope) {
            return Err(ClientError::Protocol(
                "Memory search contains another scope".into(),
            ));
        }
        Ok(result)
    }

    pub fn delete_memory(
        &mut self,
        params: MemoryDeleteParams,
    ) -> Result<memories::MemoryDeleteResult, ClientError> {
        let expected = (params.memory_id.clone(), params.scope.clone());
        let result: memories::MemoryDeleteResult = self.call(ClientMethod::MemoryDelete, params)?;
        if result.memory_id != expected.0 || result.scope != expected.1 {
            return Err(ClientError::Protocol(
                "Memory deletion returned another identity".into(),
            ));
        }
        Ok(result)
    }
}
