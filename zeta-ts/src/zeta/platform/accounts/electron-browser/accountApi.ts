import type { AccountLoginCancelResult, AccountLoginStartResult, AccountLogoutResult, AccountReadResult } from '../../../../../generated/app-server/types.js';
import { invoke } from '../../ipc/electron-browser/rendererIpc.js';
import type { IAccountApi } from '../common/accountApi.js';

export function createAccountApi(): IAccountApi {
	return {
		read: () => invoke<AccountReadResult>('zeta:accounts:read'),
		startLogin: params => invoke<AccountLoginStartResult>('zeta:accounts:login-start', params),
		cancelLogin: params => invoke<AccountLoginCancelResult>('zeta:accounts:login-cancel', params),
		logout: params => invoke<AccountLogoutResult>('zeta:accounts:logout', params),
	};
}
