import {Component, html, Backend, Model, timeout} from 'veda-client';
import Literal from './Literal.js';
import state from './State.js';

export default class DocumentProcessingPipelinesList extends Component(HTMLElement) {
  static tag = 'bpa-document-processing-pipelines-list';

  async added () {
    state.on('documentProcessingPipelines', this.up);

    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:RunningDocumentProcessingPipelines';
    params['v-s:resultFormat'] = 'cols';
    try {
      const {id: pipelines} = await Backend.stored_query(params);
      state.documentProcessingPipelines = pipelines;
    } catch (e) {
      console.error('Ошибка при запросе документов в обработке', e);
      state.documentProcessingPipelines = [];
    }
  }

  up = (pipelines) => {
    this.pipelines = pipelines;
    this.update();
  }

  removed() {
    state.off('documentProcessingPipelines', this.up);
  }

  render() {
    if (!this.pipelines?.length) {
      return html`<div></div>`;
    }
    return html`
      <div class="sheet">
        <div class="table-responsive">
          <table class="table mb-0">
            <tbody>
              ${this.pipelines?.map(id => html`
                <tr about="${id}" is="${PipelineRow}"></tr>
              `).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `;
  }
}
customElements.define(DocumentProcessingPipelinesList.tag, DocumentProcessingPipelinesList);

const debounce = (func, wait) => {
  let timeout;
  return function(...args) {
    clearTimeout(timeout);
    timeout = setTimeout(() => func.apply(this, args), wait);
  };
};

class PipelineRow extends Component(HTMLTableRowElement) {
  static tag = 'bpa-pipeline-row';

  async added() {
    this.model.on('modified', this.handler);
    this.model.on('v-bpa:estimatedTime', this.updateProgress);
  }

  updateProgress = async (arg) => {
    let modelEstimatedTime;
    if (arg instanceof Array) {
      modelEstimatedTime = arg[0] * 1000;
    }
    const now = Date.now();
    this.started = this.started || now;
    this.progress = this.progress || 0;
    const modelProgress = (this.model['v-bpa:percentComplete']?.[0] ?? 0) / 100;
    this.estimatedTime = this.estimatedTime || modelEstimatedTime || Infinity;
    const elapsed = now - this.started;

    if (modelEstimatedTime) {
      this.estimatedTime = elapsed + modelEstimatedTime;
    }
    this.progress = Math.min(0.99, Math.max(this.progress, modelProgress, elapsed / this.estimatedTime));

    this.update();

    if (this.progress < 0.99) {
      await timeout(1000);
      requestAnimationFrame(this.updateProgress);
    }
  }

  handler = debounce(async () => {
    this.update();
    if (this.model.hasValue('v-bpa:hasExecutionState', 'v-bpa:ExecutionCompleted')) {
      state.emit('document-processing-pipeline-completed', this.model.id);
    }
  }, 100);

  removed() {
    this.model.off('modified', this.handler);
    this.model.off('v-bpa:estimatedTime', this.updateProgress);
  }

  render() {
    const inProgress = this.model.hasValue('v-bpa:hasExecutionState', 'v-bpa:ExecutionInProgress');
    const hasError = this.model.hasValue('v-bpa:hasExecutionState', 'v-bpa:ExecutionError');
    const isCompleted = this.model.hasValue('v-bpa:hasExecutionState', 'v-bpa:ExecutionCompleted');
    const lastError = this.model['v-bpa:lastError']?.[0] || '';
    const progress = this.progress * 100 || (this.model['v-bpa:percentComplete']?.[0] ?? 0);

    return html`
      <td width="45%">
        <div class="d-flex align-items-center">
          <i class="bi bi-file-earmark-text me-2"></i>
          <span rel="v-s:attachment">
            <span property="v-s:fileName"></span>
          </span>
        </div>
      </td>
      <td width="30%" class="align-middle">
        ${inProgress
          ? html`
            <div class="progress border border-tertiary-subtle" style="height:20px;">
              <div class="progress-bar progress-bar-striped bg-success fw-bold"
                role="progressbar"
                style="width:${progress}%"
                aria-valuenow="${Math.floor(progress)}"
                aria-valuemin="0"
                aria-valuemax="100">
                ${Math.floor(progress)}%
              </div>
            </div>`
          : hasError
          ? html`<span class="badge bg-danger" title="${lastError}" about="v-bpa:Error" property="rdfs:label"></span>`
          : ''
        }
      </td>
      <td width="25%" class="text-end" rel="v-bpa:hasExecutionState">
        ${isCompleted
          ? html`<span class="bi bi-check-circle-fill text-success"></span>`
          : hasError
          ? html`<span class="bi bi-exclamation-circle-fill text-danger"></span>`
          : inProgress
          ? html`<span class="bi bi-clock-history text-secondary"></span>`
          : ''}
        <span class="ms-1" property="rdfs:label"></span>
      </td>
    `;
  }
}
customElements.define(PipelineRow.tag, PipelineRow, {extends: 'tr'});
