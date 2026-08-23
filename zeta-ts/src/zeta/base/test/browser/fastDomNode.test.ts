import assert from 'node:assert/strict';
import test from 'node:test';
import { FastDomNode } from '../../browser/fastDomNode.js';

const styleProperties = ['width', 'height', 'top', 'left', 'right', 'bottom', 'lineHeight', 'transform', 'boxShadow'] as const;
const cachedProperties = [...styleProperties, 'className', 'textContent'] as const;
type StyleProperty = typeof styleProperties[number];
type CachedProperty = typeof cachedProperties[number];

test('FastDomNode writes each changed value once', () => {
	const fixture = createElementFixture();
	const node = new FastDomNode(fixture.element);

	node.setWidth(24);
	node.setWidth('24px');
	node.setHeight('50%');
	node.setHeight('50%');
	node.setTop(8);
	node.setTop(8);
	node.setLeft('2rem');
	node.setLeft('2rem');
	node.setRight(12);
	node.setRight('12px');
	node.setBottom('4%');
	node.setBottom('4%');
	node.setLineHeight(20);
	node.setLineHeight('20px');
	node.setTransform('translate3d(0, 8px, 0)');
	node.setTransform('translate3d(0, 8px, 0)');
	node.setBoxShadow('1px 0 red inset');
	node.setBoxShadow('1px 0 red inset');
	node.setClassName('decoration');
	node.setClassName('decoration');
	node.setTextContent('marker');
	node.setTextContent('marker');
	node.setHidden(true);
	node.setHidden(true);
	node.setTabIndex(0);
	node.setTabIndex(0);

	assert.deepEqual(fixture.values, {
		width: '24px',
		height: '50%',
		top: '8px',
		left: '2rem',
		right: '12px',
		bottom: '4%',
		lineHeight: '20px',
		transform: 'translate3d(0, 8px, 0)',
		boxShadow: '1px 0 red inset',
		className: 'decoration',
		textContent: 'marker',
	});
	assert.deepEqual(fixture.writes, {
		width: 1,
		height: 1,
		top: 1,
		left: 1,
		right: 1,
		bottom: 1,
		lineHeight: 1,
		transform: 1,
		boxShadow: 1,
		className: 1,
		textContent: 1,
	});
	assert.deepEqual(fixture.hidden, { value: true, writes: 1 });
	assert.deepEqual(fixture.tabIndex, { value: 0, writes: 1 });
});

test('FastDomNode keeps class writes coherent across set and toggle operations', () => {
	const fixture = createElementFixture();
	fixture.values.className = 'retained';
	const node = new FastDomNode(fixture.element);

	node.toggleClassName('active', true);
	node.toggleClassName('active', true);
	node.setClassName('retained active');
	node.toggleClassName('active', false);
	node.toggleClassName('active', false);
	node.setClassName('retained');
	node.toggleClassName('selected');
	node.toggleClassName('selected');

	assert.equal(fixture.values.className, 'retained');
	assert.equal(fixture.writes.className, 4);
});

test('FastDomNode starts from the node current inline values', () => {
	const fixture = createElementFixture();
	fixture.values.width = '24px';
	fixture.values.height = '50%';
	fixture.values.top = '8px';
	fixture.values.left = '2rem';
	fixture.values.right = '12px';
	fixture.values.bottom = '4%';
	fixture.values.lineHeight = '20px';
	fixture.values.transform = 'translate3d(0, 8px, 0)';
	fixture.values.boxShadow = '1px 0 red inset';
	fixture.values.className = 'decoration';
	fixture.values.textContent = 'marker';
	fixture.hidden.value = true;
	fixture.tabIndex.value = 0;
	const node = new FastDomNode(fixture.element);

	node.setWidth(24);
	node.setHeight('50%');
	node.setTop(8);
	node.setLeft('2rem');
	node.setRight(12);
	node.setBottom('4%');
	node.setLineHeight(20);
	node.setTransform('translate3d(0, 8px, 0)');
	node.setBoxShadow('1px 0 red inset');
	node.setClassName('decoration');
	node.setTextContent('marker');
	node.setHidden(true);
	node.setTabIndex(0);

	assert.deepEqual(fixture.writes, createPropertyRecord(0));
	assert.equal(fixture.hidden.writes, 0);
	assert.equal(fixture.tabIndex.writes, 0);
});

test('FastDomNode tracks cleared inline values before restoring geometry', () => {
	const fixture = createElementFixture();
	const node = new FastDomNode(fixture.element);

	node.setLeft(8);
	node.setLeft('');
	node.setLeft(8);

	assert.equal(fixture.values.left, '8px');
	assert.equal(fixture.writes.left, 3);
});

function createElementFixture(): {
	readonly element: HTMLElement;
	readonly values: Record<CachedProperty, string>;
	readonly writes: Record<CachedProperty, number>;
	readonly hidden: { value: boolean; writes: number };
	readonly tabIndex: { value: number; writes: number };
} {
	const values = createPropertyRecord('');
	const writes = createPropertyRecord(0);
	const hidden = { value: false, writes: 0 };
	const tabIndex = { value: -1, writes: 0 };
	const style = {} as CSSStyleDeclaration;
	for (const property of styleProperties) {
		Object.defineProperty(style, property, {
			get: () => values[property],
			set: (value: string) => {
				values[property] = value;
				writes[property]++;
			},
		});
	}
	const element = { style } as HTMLElement;
	Object.defineProperty(element, 'className', {
		get: () => values.className,
		set: (value: string) => {
			values.className = value;
			writes.className++;
		},
	});
	Object.defineProperty(element, 'classList', {
		value: {
			toggle: (token: string, force?: boolean): boolean => {
				const classNames = values.className.split(/\s+/u).filter(Boolean);
				const index = classNames.indexOf(token);
				const shouldHaveIt = force ?? index === -1;
				if (shouldHaveIt === (index !== -1)) {
					return shouldHaveIt;
				}
				if (shouldHaveIt) {
					classNames.push(token);
				} else {
					classNames.splice(index, 1);
				}
				values.className = classNames.join(' ');
				writes.className++;
				return shouldHaveIt;
			},
		},
	});
	Object.defineProperty(element, 'textContent', {
		get: () => values.textContent,
		set: (value: string) => {
			values.textContent = value;
			writes.textContent++;
		},
	});
	Object.defineProperty(element, 'hidden', {
		get: () => hidden.value,
		set: (value: boolean) => {
			hidden.value = value;
			hidden.writes++;
		},
	});
	Object.defineProperty(element, 'tabIndex', {
		get: () => tabIndex.value,
		set: (value: number) => {
			tabIndex.value = value;
			tabIndex.writes++;
		},
	});
	return { element, values, writes, hidden, tabIndex };
}

function createPropertyRecord<TValue>(value: TValue): Record<CachedProperty, TValue> {
	return {
		width: value,
		height: value,
		top: value,
		left: value,
		right: value,
		bottom: value,
		lineHeight: value,
		transform: value,
		boxShadow: value,
		className: value,
		textContent: value,
	};
}
