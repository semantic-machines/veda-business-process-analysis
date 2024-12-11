import {Component, html, Backend, Model} from 'veda-client';
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
                <tr about="${id}">
                  <td width="55%">
                    <div class="d-flex align-items-center">
                      <i class="bi bi-file-earmark-text me-2"></i>
                      <span about="${id}" rel="v-s:attachment">
                        <span property="v-s:fileName"></span>
                      </span>
                    </div>
                  </td>
                  <td width="15%">
                    <${Literal} about="${id}" property="v-bpa:currentStage"></${Literal}>
                  </td>
                  <td width="30%" class="text-end" about="${id}" rel="v-bpa:hasExecutionState">
                    <div class="d-flex align-items-center justify-content-end">
                      <div class="spinner-grow spinner-grow-sm text-secondary me-2" role="status"></div>
                      <span property="rdfs:label"></span>
                    </div>
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
