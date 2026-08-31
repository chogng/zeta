import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import ts from '../../../../zeta-ts/node_modules/typescript/lib/typescript.js';

const repositoryRoot = resolve(import.meta.dirname, '../../../..');
const zetaEditorRoot = resolve(repositoryRoot, 'zeta-ts/src/zeta/editor');
const vscodeEditorRoot = resolve(repositoryRoot, '../vscode/src/vs/editor');
const ledgerPath = resolve(zetaEditorRoot, 'api-alignment-status.md');
const ledger = readFileSync(ledgerPath, 'utf8');
const declarations = readDeclarations(ledger, '尚未补齐的同名契约');

const results = declarations.map(({ file, declaration }) => {
	const localFile = resolve(zetaEditorRoot, file);
	const upstreamFile = resolve(vscodeEditorRoot, file);
	if (!existsSync(localFile) || !existsSync(upstreamFile)) {
		return {
			file,
			declaration,
			kind: !existsSync(localFile) ? 'missing-local-file' : 'missing-upstream-file',
			missing: [],
			extra: [],
		};
	}
	const local = readDeclaration(localFile, declaration);
	const upstream = readDeclaration(upstreamFile, declaration);
	if (!local || !upstream) {
		return {
			file,
			declaration,
			kind: !local ? 'missing-local-declaration' : 'missing-upstream-declaration',
			missing: [],
			extra: [],
		};
	}
	const missing = upstream.members.filter(member => !local.members.includes(member));
	const extra = local.members.filter(member => !upstream.members.includes(member));
	return { file, declaration, kind: `${local.kind}/${upstream.kind}`, missing, extra };
}).sort((left, right) => {
	const difference = value => value.missing.length + value.extra.length;
	return difference(left) - difference(right) || left.file.localeCompare(right.file) || left.declaration.localeCompare(right.declaration);
});

for (const result of results) {
	const differenceCount = result.missing.length + result.extra.length;
	process.stdout.write(`${String(differenceCount).padStart(3)} ${result.file}::${result.declaration} [${result.kind}]\n`);
	if (result.missing.length > 0) process.stdout.write(`    missing: ${result.missing.join(', ')}\n`);
	if (result.extra.length > 0) process.stdout.write(`    extra:   ${result.extra.join(', ')}\n`);
}

function readDeclaration(file, declarationName) {
	const source = ts.createSourceFile(file, readFileSync(file, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
	const declarations = source.statements
		.filter(node => hasName(node, declarationName))
		.map(node => ({ kind: ts.SyntaxKind[node.kind], members: readMembers(node) }));
	const preferred = declarations.filter(declaration => declaration.kind !== 'VariableDeclaration');
	const candidates = preferred.length > 0 ? preferred : declarations;
	if (candidates.length !== 1) return undefined;
	return candidates[0];

}

function hasName(node, name) {
	if (!('name' in node) || !node.name) return false;
	return node.name.getText() === name;
}

function readMembers(node) {
	if (ts.isFunctionDeclaration(node)) return [`(${node.parameters.length})`];
	if (ts.isVariableDeclaration(node)) return readObjectMembers(node.initializer);
	if (!('members' in node) || !node.members) return [];
	return node.members.flatMap(member => {
		const name = memberName(member);
		if (!ts.isConstructorDeclaration(member)) return name ? [name] : [];
		const properties = member.parameters
			.filter(parameter => ts.isParameterPropertyDeclaration(parameter, member))
			.map(parameter => parameter.name.getText());
		return [name, ...properties];
	}).filter(Boolean).sort();
}

function readObjectMembers(initializer) {
	if (!initializer || !ts.isObjectLiteralExpression(initializer)) return [];
	return initializer.properties.map(memberName).filter(Boolean).sort();
}

function memberName(member) {
	if (ts.isConstructorDeclaration(member)) return 'constructor';
	if (!('name' in member) || !member.name) return undefined;
	return member.name.getText();
}

function readDeclarations(markdown, heading) {
	const start = markdown.indexOf(`## ${heading}`);
	if (start < 0) throw new Error(`Missing ledger heading: ${heading}`);
	const sectionStart = markdown.indexOf('\n', start);
	const nextHeading = markdown.indexOf('\n## ', sectionStart + 1);
	const section = markdown.slice(sectionStart, nextHeading < 0 ? markdown.length : nextHeading);
	const result = [];
	for (const line of section.split(/\r?\n/u)) {
		if (!line.startsWith('| `')) continue;
		const cells = line.split('|').slice(1, -1).map(cell => cell.trim());
		if (cells.length < 2) continue;
		const file = unwrapCode(cells[0]);
		for (const declaration of cells[1].split('、').map(unwrapCode)) result.push({ file, declaration });
	}
	return result;
}

function unwrapCode(value) {
	return value.startsWith('`') && value.endsWith('`') ? value.slice(1, -1) : value;
}
