import {Component, html} from 'veda-client';
import ProcessJustificationIndicator from './ProcessJustificationIndicator.js';

export default class ProcessDocumentInfo extends Component(HTMLElement) {
  static tag = 'bpa-process-document-info';

  render() {
    const documentsCount = this.model['v-bpa:hasProcessDocument']?.length;
    return html`
      ${documentsCount 
        ? html`
            <a href="#process-document-modal" data-bs-toggle="modal" data-bs-target="#process-document-modal" class="text-secondary" style="cursor:pointer">
              <span about="v-bpa:ProcessDocuments" property="rdfs:label"></span>: ${documentsCount}
            </a>
            <div class="modal fade" id="process-document-modal">
              <div class="modal-dialog modal-dialog-centered">
                <div class="modal-content">
                  <div class="modal-body">
                    <div class="d-flex justify-content-between">
                      <div class="fs-5 mb-2" about="${this.model.id}" rel="v-bpa:hasProcessJustification">
                        <${ProcessJustificationIndicator} about="{{this.model.id}}" property="rdfs:comment"></${ProcessJustificationIndicator}>
                      </div>
                      <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
                    </div>
                    <div property="v-bpa:justificationReason" class="mb-3"></div>
                    <${ProcessDocumentList} about="${this.model.id}"></${ProcessDocumentList}>
                  </div>
                </div>
              </div>
            </div>             
          `
        : html`
            <a class="text-secondary">
              <span about="v-bpa:AddProcessDocument" property="rdfs:label"></span>
            </a>
          `
      }
    `;
  }
}

customElements.define(ProcessDocumentInfo.tag, ProcessDocumentInfo);

class ProcessDocumentList extends Component(HTMLElement) {
  static tag = 'bpa-process-document-list';

  render() {
    return html`
      <h5 about="v-bpa:ProcessDocuments" property="rdfs:label"></h5>
      <div about="${this.model.id}" rel="v-bpa:hasProcessDocument">
        <div class="card mb-3 bg-light border-light">
          <div class="card-body p-2">
            <div class="card-title mb-0 d-flex align-items-center">
              <i class="fs-4 bi bi-file-earmark-text text-secondary me-2"></i>
              <span property="v-bpa:documentName"></span>
              <span class="text-secondary ms-auto">{{ this.model['v-s:created']?.[0].toLocaleDateString() }}</span>
            </div>
          </div>
        </div>
      </div>
      <button type="button" class="btn btn-light">
        <i class="bi bi-plus me-1"></i>
        <span about="v-bpa:AddProcessDocument" property="rdfs:label"></span>
      </button>
    `;
  }
}

customElements.define(ProcessDocumentList.tag, ProcessDocumentList);
