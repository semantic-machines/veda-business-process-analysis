import {Component, html, Backend, Model} from 'veda-client';
import BusinessProcessCard from './BusinessProcessCard.js';

export default class BusinessProcessList extends Component(HTMLElement) {
  static tag = 'bpa-process-list';

  async added() {
    const params = new Model;
    params['rdf:type'] = 'v-s:QueryParams';
    params['v-s:storedQuery'] = 'v-bpa:AllBusinessProcesses';
    const {rows: processes} = await Backend.stored_query(params);
    this.processes = processes;
  }
  
  async render() {
    return html`
      <div class="sheet">
        <h3>Бизнес-процессы</h3>
        <hr>
        ${this.processes.map(([processId]) => html`
          <${BusinessProcessCard} about=${processId}></${BusinessProcessCard}>
        `).join('')}
      </div>
    `;
  }
}

customElements.define(BusinessProcessList.tag, BusinessProcessList);
