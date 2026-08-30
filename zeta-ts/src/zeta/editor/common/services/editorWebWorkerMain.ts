import { LanguageWorkerWireServer } from '../languages/languageWorkerWire.js';
import { startZetaWorker } from '../../zetaWorkerBootstrap.js';
import { EditorWorkerRequestExecutor } from './editorWorkerRequestExecutor.js';
import { editorWorkerWireCodec } from './editorWorkerWire.js';

startZetaWorker(({ port, resources }) => {
	resources.add(new LanguageWorkerWireServer(port, editorWorkerWireCodec, new EditorWorkerRequestExecutor()));
});
