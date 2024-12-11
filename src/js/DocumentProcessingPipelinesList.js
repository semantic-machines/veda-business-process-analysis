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
                <tr about="${id}" is="${PipelineRow}">
                  <td width="55%">
                    <div class="d-flex align-items-center">
                      <i class="bi bi-file-earmark-text me-2"></i>
                      <span about="${id}" rel="v-s:attachment">
                        <span property="v-s:fileName"></span>
                      </span>
                    </div>
                  </td>
                  <td width="15%" class="align-middle">
                    <div class="progress border border-tertiary-subtle" style="height: 16px;">
                      <div class="progress-bar fw-bold progress-bar-striped progress-bar-animated bg-success"
                        role="progressbar"
                        style="width:{{this.model?.['v-bpa:percentComplete']?.[0] || 0}}%"
                        aria-valuenow="{{this.model?.['v-bpa:percentComplete']?.[0] || 0}}"
                        aria-valuemin="0"
                        aria-valuemax="100">
                        {{this.model?.['v-bpa:percentComplete']?.[0] || 0}}%
                      </div>
                    </div>
                  </td>
                  <td width="30%" class="text-end" about="${id}" rel="v-bpa:hasExecutionState">
                    <span property="rdfs:label"></span>
                  </td>
                </tr>
              `).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `;
  }
}
customElements.define(DocumentProcessingPipelinesList.tag, DocumentProcessingPipelinesList);

class PipelineRow extends Component(HTMLTableRowElement) {
  static tag = 'bpa-pipeline-row';

  async added() {
    this.model.on('modified', this.handler);
  }

  handler = async () => {
    this.update();
    if (this.model.hasValue('v-bpa:hasExecutionState', 'v-bpa:ExecutionCompleted')) {
      state.emit('document-processing-pipeline-completed', this.model.id);
      // await timeout(5000);
      // state.documentProcessingPipelines = state.documentProcessingPipelines.filter(id => id !== this.model.id);
    }
  }

  removed() {
    this.model.off('modified', this.onCompleted);
  }
}
customElements.define(PipelineRow.tag, PipelineRow, {extends: 'tr'});

