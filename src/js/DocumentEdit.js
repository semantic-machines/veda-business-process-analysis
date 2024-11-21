import {Component, html, Model} from 'veda-client';
import InputText from './controls/InputText.js';
import Textarea from './controls/Textarea.js';
import InputAudio from './controls/InputAudio.js';

export default class DocumentEdit extends Component(HTMLElement) {
  static tag = 'bpa-document-edit';

  added() {
    if (!this.model) {
      this.model = new Model;
      this.model['rdf:type'] = 'v-bpa:ProcessDocument';
    }
  }

  async save() {
    await this.model.save();
    location.hash = `#/DocumentView/${this.model.id}`;
  }

  async cancel() {
    if (this.model.isNew()) {
      history.back();
    } else {
      await this.model.reset();
      location.hash = `#/DocumentView/${this.model.id}`;
    }
  }

  render() {
    return html`
      <div class="sheet">
        <h3 class="mb-1">
          <i class="bi bi-file-earmark-text me-2"></i>
          <span about="v-bpa:ProcessDocument" property="rdfs:label"></span>
        </h3>
        <div class="mb-3">
          <label class="form-label fw-bold" about="v-bpa:documentName" property="rdfs:label"></label>
          <input type="text" class="form-control" is="${InputText}" about="${this.model.id}" data-property="v-bpa:documentName">
        </div>
        <div class="mb-3 position-relative">
          <label class="form-label fw-bold" about="v-bpa:documentContent" property="rdfs:label"></label>
          <textarea id="document-content" class="form-control" is="${Textarea}" about="${this.model.id}" data-property="v-bpa:documentContent" rows="10"></textarea>
          <div class="position-absolute bottom-0" style="right:0.75rem;">
            <${InputAudio} data-for="document-content"></${InputAudio}>
          </div>
        </div>
      </div>
      <div class="d-flex justify-content-start gap-2 mt-3">
        <button @click="${(e) => this.save(e)}" class="btn btn-success">
          <span about="v-bpa:Save" property="rdfs:label"></span>
        </button>
        <button @click="${(e) => this.cancel(e)}" class="btn btn-link text-muted text-decoration-none">
          <span about="v-bpa:Cancel" property="rdfs:label"></span>
        </button>
      </div>
      `;
  }
}

customElements.define(DocumentEdit.tag, DocumentEdit);
