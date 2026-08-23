import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { Account, AccountLoginCompletion, AccountLoginMethod, AccountState, IAccountService } from '../../../../platform/accounts/common/accountService.js';

interface SubscriptionAccountCardOptions {
	readonly providerId: string;
	readonly title: string;
	readonly productName: string;
	readonly signedOutCopy: string;
	readonly loginMethod: AccountLoginMethod;
}

/** One provider-scoped subscription login card backed by the shared account service. */
export class SubscriptionAccountCard extends DisposableOwner {
	readonly element: HTMLElement;
	private readonly summary: HTMLParagraphElement;
	private readonly action: HTMLButtonElement;
	private readonly challenge: HTMLParagraphElement;
	private account: Account | undefined;
	private activeLoginId: string | undefined;
	private disposed = false;

	constructor(container: HTMLElement, private readonly accounts: IAccountService, private readonly options: SubscriptionAccountCardOptions) {
		super();
		const document = container.ownerDocument;
		this.element = h(document, 'section');
		this.element.className = 'zeta-model-settings-account';
		this.element.dataset.provider = options.providerId;
		const copy = h(document, 'div');
		copy.className = 'zeta-model-settings-account-copy';
		const title = h(document, 'h3');
		title.textContent = options.title;
		this.summary = h(document, 'p');
		this.summary.textContent = `Checking local ${options.productName} sign-in…`;
		this.challenge = h(document, 'p');
		this.challenge.className = 'zeta-model-settings-account-challenge';
		this.challenge.hidden = true;
		copy.append(title, this.summary, this.challenge);
		this.action = h(document, 'button');
		this.action.type = 'button';
		this.action.className = 'zeta-model-settings-account-action';
		this.action.textContent = 'Sign in';
		this.element.append(copy, this.action);
		container.append(this.element);
		this.own(addDisposableListener(this.action, 'click', () => void this.runAction()));
		this.own(accounts.onDidChangeAccounts(state => this.acceptState(state)));
		this.own(accounts.onDidCompleteLogin(completion => this.completeLogin(completion)));
		void this.load();
		this.defer(() => {
			this.disposed = true;
			this.element.remove();
		});
	}

	private async load(): Promise<void> {
		try {
			this.acceptState(await this.accounts.read());
		} catch (error) {
			if (this.disposed) return;
			this.summary.textContent = error instanceof Error
				? `${this.options.productName} account unavailable: ${error.message}`
				: `${this.options.productName} account is unavailable.`;
		}
	}

	private acceptState(state: AccountState): void {
		if (this.disposed) return;
		this.account = state.accounts.find(account => account.provider === this.options.providerId);
		if (!this.activeLoginId) this.render();
	}

	private render(): void {
		this.action.disabled = false;
		this.challenge.hidden = true;
		if (!this.account) {
			this.summary.textContent = this.options.signedOutCopy;
			this.action.textContent = 'Sign in';
			return;
		}
		const identity = this.account.displayName ?? this.account.email ?? this.account.accountId;
		const plan = this.account.plan ? ` · ${this.account.plan}` : '';
		if (this.account.status === 'ready') {
			this.summary.textContent = `Signed in as ${identity}${plan}`;
			this.action.textContent = 'Sign out';
		} else {
			this.summary.textContent = `${this.options.productName} needs authorization for ${identity}${plan}`;
			this.action.textContent = 'Sign in again';
		}
	}

	private async runAction(): Promise<void> {
		if (this.activeLoginId) {
			const loginId = this.activeLoginId;
			this.action.disabled = true;
			try {
				await this.accounts.cancelLogin(loginId);
				if (this.activeLoginId === loginId) this.activeLoginId = undefined;
				this.render();
			} catch (error) {
				this.showError(error, `Unable to cancel ${this.options.productName} sign-in.`);
			} finally {
				this.action.disabled = false;
			}
			return;
		}
		if (this.account?.status === 'ready') {
			this.action.disabled = true;
			try {
				await this.accounts.logout(this.options.providerId);
				this.account = undefined;
				this.render();
			} catch (error) {
				this.showError(error, `Unable to sign out of ${this.options.productName}.`);
			} finally {
				this.action.disabled = false;
			}
			return;
		}
		this.action.disabled = true;
		this.summary.textContent = `Starting ${this.options.productName} sign-in…`;
		try {
			const started = await this.accounts.startLogin(this.options.loginMethod);
			if (started.type !== 'deviceCode') throw new Error(`${this.options.productName} did not return a device-code challenge`);
			this.activeLoginId = started.loginId;
			this.summary.textContent = `${this.options.productName} opened in your browser. Enter the copied code to authorize Zeta.`;
			this.challenge.textContent = `Code: ${started.userCode}`;
			this.challenge.hidden = false;
			this.action.textContent = 'Cancel';
		} catch (error) {
			this.activeLoginId = undefined;
			this.showError(error, `Unable to start ${this.options.productName} sign-in.`);
		}
		this.action.disabled = false;
	}

	private completeLogin(completion: AccountLoginCompletion): void {
		if (completion.loginId !== this.activeLoginId) return;
		this.activeLoginId = undefined;
		this.account = completion.account.accounts.find(account => account.provider === this.options.providerId);
		if (completion.status.type === 'failed') {
			this.showError(new Error(completion.status.failure.message), `${this.options.productName} sign-in failed.`);
			this.action.textContent = 'Sign in';
			this.action.disabled = false;
			return;
		}
		this.render();
	}

	private showError(error: unknown, fallback: string): void {
		this.summary.textContent = error instanceof Error ? error.message : fallback;
		this.challenge.hidden = true;
	}
}
