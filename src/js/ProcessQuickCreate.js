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
    this.restoreValue();
  }

  create = async () => {
    try {
      this.showSpinner(true);
      await this.createProcess();
    } catch (error) {
      this.handleError(error);
    } finally {
      this.showSpinner(false);
    }
  }

  createProcess = async () => {
    this.model.isSync(false);
    await this.model.save();
    await this.waitForProcessResult();
  }

  waitForProcessResult = async () => {
    return Promise.race([
      this.handleProcessResult(),
      this.createTimeout()
    ]);
  }

  handleProcessResult = () => {
    return new Promise((resolve, reject) => {
      const handleReset = async () => {
        if (!this.model.hasValue('v-bpa:hasResult')) return;
        
        try {
          const result = await this.model['v-bpa:hasResult'][0].load();
          const newProcess = this.prepareNewProcess(result);
          this.manualCreate(newProcess);
          resolve();
        } catch (error) {
          reject(error);
        } finally {
          this.model.off('afterreset', handleReset);
        }
      }
      
      this.model.on('afterreset', handleReset);
    });
  }

  prepareNewProcess = (result) => {
    const json = JSON.parse(JSON.stringify(result));
    json['@'] = genUri();
    json['rdf:type'] = json['v-bpa:targetType'];
    delete json['v-bpa:targetType'];

    const newProcess = new Model(json);
    newProcess.isNew(true);
    newProcess.isSync(false);
    return newProcess;
  }

  createTimeout = () => {
    return timeout(30000).then(() => {
      throw new Error('Превышено время ожидания обработки запроса');
    });
  }

  handleError = (error) => {
    alert(`Ошибка создания свободного описания процесса: ${error.message}`);
    console.error('Ошибка создания свободного описания процесса', error);
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
          <textarea class="form-control" placeholder="Введите текст с клавиатуры или воспользуйтесь микрофоном" 
            is="${Textarea}" about="${this.model.id}" data-property="v-bpa:rawInput" rows="7"
            @input="storeValue">
          </textarea>
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
