import {Emitter, Observable} from 'veda-client';

const State = Observable(Emitter(Object))
const state = new State();

export default state;