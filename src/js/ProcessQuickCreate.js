// Start of Selection
import {Component, html, Model, genUri, decorator, timeout} from 'veda-client';
import Textarea from './controls/Textarea.js';
import InputAudio from './controls/InputAudio.js';

export default class ProcessQuickCreate extends Component(HTMLElement) {
  static tag = 'bpa-process-quick-create';

  storeValue(e) {
    sessionStorage.setItem('ProcessQuickCreate_rawInput', e.target.value);
  }

  restoreValue() {
    const savedText = sessionStorage.getItem('ProcessQuickCreate_rawInput');
    if (savedText) {
      this.model['v-bpa:rawInput'] = [savedText];
    }
  }

  clearValue() {
    sessionStorage.removeItem('ProcessQuickCreate_rawInput');
  }

  added() {
    this.model = new Model;
    this.model['rdf:type'] = 'v-bpa:GenericProcessingRequest';
    this.model['v-bpa:prompt'] = 'v-bpa:CreateBusinessProcessPrompt';
    this.model.on('afterreset', this.handleReset);
    this.restoreValue();
  }

  removed() {
    this.model.off('afterreset', this.handleReset);
  }

  create = async () => {
    this.showSpinner(true);

    try {
      this.model.isSync(false);
      await this.model.save();
    } catch (error) {
      alert(`Ошибка создания свободного описания процесса: ${error.message}`);
      console.error('Ошибка создания свободного описания процесса', error);      
      this.showSpinner(false);
    }
  }
  
  handleReset = async () => {
    if (!this.model.hasValue('v-bpa:hasResult')) return;

    try {
      const result = await this.model['v-bpa:hasResult'][0].load();
      const json = JSON.parse(JSON.stringify(result));
      json['@'] = genUri();
      json['rdf:type'] = json['v-bpa:targetType'];
      delete json['v-bpa:targetType'];
      
      const newProcess = new Model(json);
      newProcess.isNew(true);
      this.manualCreate(newProcess);
    } catch (error) {
      alert(`Ошибка загрузки результата заполнения: ${error.message}`);
      console.error('Ошибка загрузки результата заполнения', error);
    } finally {
      this.showSpinner(false);
    }
  }

  manualCreate(newProcess) {
    if (newProcess instanceof Event) {
      newProcess = new Model;
      newProcess['rdf:type'] = 'v-bpa:BusinessProcess';
    }
    location.hash = `#/ProcessEdit/${newProcess.id}`;
  }

  cancel() {
    this.clearValue();
    history.back();
  }

  showSpinner(show) {
    const createButton = this.querySelector('.create-button');
    const spinner = createButton.querySelector('.spinner-grow');
    createButton.disabled = show;
    spinner.classList.toggle('d-none', !show);
  }

  render() {
    return html`
      <div class="sheet">
        <h3 class="mb-1">
          <i class="bi bi-diagram-3 me-2"></i>
          <span about="v-bpa:ProcessQuickCreate" property="rdfs:label"></span>
        </h3>
        <p class="text-muted fw-bold" about="v-bpa:ProcessQuickCreate" property="rdfs:comment"></p>
        <div class="mb-3 position-relative">
          <textarea placeholder="Введите текст с клавиатуры или воспользуйтесь микрофоном" class="form-control" is="${Textarea}" about="${this.model.id}" data-property="v-bpa:rawInput" rows="7"
            @input="storeValue"></textarea>
          <div class="position-absolute bottom-0" style="right:0.75rem;">
            <${InputAudio} about="${this.model.id}" data-property="v-bpa:rawInput"></${InputAudio}>
          </div>
        </div>
        <div class="d-flex justify-content-between">
          <div class="d-flex gap-2">
            <button @click="create" class="btn btn-primary create-button">
              <span class="spinner-grow spinner-grow-sm me-2 d-none" aria-hidden="true"></span>
              <span about="v-bpa:Create" property="rdfs:label"></span>
            </button>
            <button @click="cancel" class="btn btn-light">
              <span about="v-bpa:Cancel" property="rdfs:label"></span>
            </button>
          </div>
          <button @click="manualCreate" class="btn btn-light">
            <span about="v-bpa:ManualCreate" property="rdfs:label"></span>
          </button>
        </div>
      </div>
    `;
  }
}

customElements.define(ProcessQuickCreate.tag, ProcessQuickCreate);
