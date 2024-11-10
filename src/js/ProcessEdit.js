import {Component, html} from 'veda-client';
import InputText from './controls/InputText.js';
import InputInteger from './controls/InputInteger.js';
import InputDecimal from './controls/InputDecimal.js';
import Textarea from './controls/Textarea.js';
import InputAudio from './controls/InputAudio.js';

export default class ProcessEdit extends Component(HTMLElement) {
  static tag = 'bpa-process-edit';

  save() {
    this.model.save();
    location.hash = `#/ProcessView/${this.model.id}`;
  }

  cancel() {
    if (this.model.isNew()) {
      history.back();
    } else {
      this.model.reset();
      location.hash = `#/ProcessView/${this.model.id}`;
    }
  }
  
  render() {
    return html`
      <div class="sheet">
        <h3 class="mb-1">
          <i class="bi bi-diagram-3 me-2"></i>
          <span about="v-bpa:BusinessProcess" property="rdfs:label"></span>
        </h3>
        <div class="row">
          <div class="col-12 col-md-9">
            <div class="mb-3">
              <label class="form-label fw-bold" about="rdfs:label" property="rdfs:label"></label>
              <input type="text" class="form-control" is="${InputText}" about="${this.model.id}" data-property="rdfs:label">
            </div>
            <div class="mb-3 position-relative">
              <label class="form-label fw-bold" about="v-bpa:processDescription" property="rdfs:label"></label>
              <textarea class="form-control" is="${Textarea}" about="${this.model.id}" data-property="v-bpa:processDescription" rows="3"></textarea>
              <div class="position-absolute bottom-0" style="right:0.75rem;">
                <${InputAudio} about="${this.model.id}" data-property="v-bpa:processDescription"></${InputAudio}>
              </div>
            </div>
            <div class="mb-3">
              <label class="form-label fw-bold" about="v-bpa:responsibleDepartment" property="rdfs:label"></label>
              <input type="text" class="form-control" is="${InputText}" about="${this.model.id}" data-property="v-bpa:responsibleDepartment">
            </div>

            <div class="mb-3">
              <label class="form-label fw-bold" about="v-bpa:processFrequency" property="rdfs:label"></label>
              <div class="input-group">
                <input type="number" is="${InputInteger}" class="form-control" about="${this.model.id}" data-property="v-bpa:processFrequency">
                <span class="input-group-text" about="v-bpa:TimesPerYear" property="rdfs:label"></span>
              </div>
            </div>

            <div class="mb-3">
              <label class="form-label fw-bold" about="v-bpa:laborCosts" property="rdfs:label"></label>
              <div class="input-group">
                <input type="number" is="${InputDecimal}" class="form-control" about="${this.model.id}" data-property="v-bpa:laborCosts">
                <span class="input-group-text" about="v-bpa:Hours" property="rdfs:label"></span>
              </div>
            </div>
          </div>
          <div class="col-12 col-md-3 border-start border-secondary-subtle">
            <div class="mb-3">
              <label class="form-label fw-bold" about="v-bpa:processParticipant" property="rdfs:label"></label>
              <textarea class="form-control" is="${Textarea}" about="${this.model.id}" data-property="v-bpa:processParticipant" rows="3"></textarea>
            </div>
          </div>
          <div class="d-flex justify-content-start gap-2 mt-3">
            <button @click="save" class="btn btn-success">
              <span about="v-bpa:Save" property="rdfs:label"></span>
            </button>
            <button @click="cancel" class="btn btn-light">
              <span about="v-bpa:Cancel" property="rdfs:label"></span>
            </button>
          </div>
        </div>
      </div>
      <div class="sheet">
        <h4 about="v-bpa:ProcessDocument" property="rdfs:label"></h4>
      </div>
    `;
  }
}

customElements.define(ProcessEdit.tag, ProcessEdit);
