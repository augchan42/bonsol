// automatically generated by the FlatBuffers compiler, do not modify

/* eslint-disable @typescript-eslint/no-unused-vars, @typescript-eslint/no-explicit-any, @typescript-eslint/no-non-null-assertion */

import * as flatbuffers from 'flatbuffers';

import { StatusTypes } from './status-types.js';


export class StatusV1 implements flatbuffers.IUnpackableObject<StatusV1T> {
  bb: flatbuffers.ByteBuffer|null = null;
  bb_pos = 0;
  __init(i:number, bb:flatbuffers.ByteBuffer):StatusV1 {
  this.bb_pos = i;
  this.bb = bb;
  return this;
}

static getRootAsStatusV1(bb:flatbuffers.ByteBuffer, obj?:StatusV1):StatusV1 {
  return (obj || new StatusV1()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

static getSizePrefixedRootAsStatusV1(bb:flatbuffers.ByteBuffer, obj?:StatusV1):StatusV1 {
  bb.setPosition(bb.position() + flatbuffers.SIZE_PREFIX_LENGTH);
  return (obj || new StatusV1()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
}

executionId():string|null
executionId(optionalEncoding:flatbuffers.Encoding):string|Uint8Array|null
executionId(optionalEncoding?:any):string|Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 4);
  return offset ? this.bb!.__string(this.bb_pos + offset, optionalEncoding) : null;
}

status():StatusTypes {
  const offset = this.bb!.__offset(this.bb_pos, 6);
  return offset ? this.bb!.readUint8(this.bb_pos + offset) : StatusTypes.Unknown;
}

mutate_status(value:StatusTypes):boolean {
  const offset = this.bb!.__offset(this.bb_pos, 6);

  if (offset === 0) {
    return false;
  }

  this.bb!.writeUint8(this.bb_pos + offset, value);
  return true;
}

proof(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 8);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

proofLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 8);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

proofArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 8);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

executionDigest(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 10);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

executionDigestLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 10);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

executionDigestArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 10);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

inputDigest(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 12);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

inputDigestLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 12);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

inputDigestArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 12);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

committedOutputs(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 14);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

committedOutputsLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 14);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

committedOutputsArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 14);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

assumptionDigest(index: number):number|null {
  const offset = this.bb!.__offset(this.bb_pos, 16);
  return offset ? this.bb!.readUint8(this.bb!.__vector(this.bb_pos + offset) + index) : 0;
}

assumptionDigestLength():number {
  const offset = this.bb!.__offset(this.bb_pos, 16);
  return offset ? this.bb!.__vector_len(this.bb_pos + offset) : 0;
}

assumptionDigestArray():Uint8Array|null {
  const offset = this.bb!.__offset(this.bb_pos, 16);
  return offset ? new Uint8Array(this.bb!.bytes().buffer, this.bb!.bytes().byteOffset + this.bb!.__vector(this.bb_pos + offset), this.bb!.__vector_len(this.bb_pos + offset)) : null;
}

exitCodeSystem():number {
  const offset = this.bb!.__offset(this.bb_pos, 18);
  return offset ? this.bb!.readUint32(this.bb_pos + offset) : 0;
}

mutate_exit_code_system(value:number):boolean {
  const offset = this.bb!.__offset(this.bb_pos, 18);

  if (offset === 0) {
    return false;
  }

  this.bb!.writeUint32(this.bb_pos + offset, value);
  return true;
}

exitCodeUser():number {
  const offset = this.bb!.__offset(this.bb_pos, 20);
  return offset ? this.bb!.readUint32(this.bb_pos + offset) : 0;
}

mutate_exit_code_user(value:number):boolean {
  const offset = this.bb!.__offset(this.bb_pos, 20);

  if (offset === 0) {
    return false;
  }

  this.bb!.writeUint32(this.bb_pos + offset, value);
  return true;
}

static startStatusV1(builder:flatbuffers.Builder) {
  builder.startObject(9);
}

static addExecutionId(builder:flatbuffers.Builder, executionIdOffset:flatbuffers.Offset) {
  builder.addFieldOffset(0, executionIdOffset, 0);
}

static addStatus(builder:flatbuffers.Builder, status:StatusTypes) {
  builder.addFieldInt8(1, status, StatusTypes.Unknown);
}

static addProof(builder:flatbuffers.Builder, proofOffset:flatbuffers.Offset) {
  builder.addFieldOffset(2, proofOffset, 0);
}

static createProofVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startProofVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addExecutionDigest(builder:flatbuffers.Builder, executionDigestOffset:flatbuffers.Offset) {
  builder.addFieldOffset(3, executionDigestOffset, 0);
}

static createExecutionDigestVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startExecutionDigestVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addInputDigest(builder:flatbuffers.Builder, inputDigestOffset:flatbuffers.Offset) {
  builder.addFieldOffset(4, inputDigestOffset, 0);
}

static createInputDigestVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startInputDigestVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addCommittedOutputs(builder:flatbuffers.Builder, committedOutputsOffset:flatbuffers.Offset) {
  builder.addFieldOffset(5, committedOutputsOffset, 0);
}

static createCommittedOutputsVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startCommittedOutputsVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addAssumptionDigest(builder:flatbuffers.Builder, assumptionDigestOffset:flatbuffers.Offset) {
  builder.addFieldOffset(6, assumptionDigestOffset, 0);
}

static createAssumptionDigestVector(builder:flatbuffers.Builder, data:number[]|Uint8Array):flatbuffers.Offset {
  builder.startVector(1, data.length, 1);
  for (let i = data.length - 1; i >= 0; i--) {
    builder.addInt8(data[i]!);
  }
  return builder.endVector();
}

static startAssumptionDigestVector(builder:flatbuffers.Builder, numElems:number) {
  builder.startVector(1, numElems, 1);
}

static addExitCodeSystem(builder:flatbuffers.Builder, exitCodeSystem:number) {
  builder.addFieldInt32(7, exitCodeSystem, 0);
}

static addExitCodeUser(builder:flatbuffers.Builder, exitCodeUser:number) {
  builder.addFieldInt32(8, exitCodeUser, 0);
}

static endStatusV1(builder:flatbuffers.Builder):flatbuffers.Offset {
  const offset = builder.endObject();
  return offset;
}

static finishStatusV1Buffer(builder:flatbuffers.Builder, offset:flatbuffers.Offset) {
  builder.finish(offset);
}

static finishSizePrefixedStatusV1Buffer(builder:flatbuffers.Builder, offset:flatbuffers.Offset) {
  builder.finish(offset, undefined, true);
}

static createStatusV1(builder:flatbuffers.Builder, executionIdOffset:flatbuffers.Offset, status:StatusTypes, proofOffset:flatbuffers.Offset, executionDigestOffset:flatbuffers.Offset, inputDigestOffset:flatbuffers.Offset, committedOutputsOffset:flatbuffers.Offset, assumptionDigestOffset:flatbuffers.Offset, exitCodeSystem:number, exitCodeUser:number):flatbuffers.Offset {
  StatusV1.startStatusV1(builder);
  StatusV1.addExecutionId(builder, executionIdOffset);
  StatusV1.addStatus(builder, status);
  StatusV1.addProof(builder, proofOffset);
  StatusV1.addExecutionDigest(builder, executionDigestOffset);
  StatusV1.addInputDigest(builder, inputDigestOffset);
  StatusV1.addCommittedOutputs(builder, committedOutputsOffset);
  StatusV1.addAssumptionDigest(builder, assumptionDigestOffset);
  StatusV1.addExitCodeSystem(builder, exitCodeSystem);
  StatusV1.addExitCodeUser(builder, exitCodeUser);
  return StatusV1.endStatusV1(builder);
}

unpack(): StatusV1T {
  return new StatusV1T(
    this.executionId(),
    this.status(),
    this.bb!.createScalarList<number>(this.proof.bind(this), this.proofLength()),
    this.bb!.createScalarList<number>(this.executionDigest.bind(this), this.executionDigestLength()),
    this.bb!.createScalarList<number>(this.inputDigest.bind(this), this.inputDigestLength()),
    this.bb!.createScalarList<number>(this.committedOutputs.bind(this), this.committedOutputsLength()),
    this.bb!.createScalarList<number>(this.assumptionDigest.bind(this), this.assumptionDigestLength()),
    this.exitCodeSystem(),
    this.exitCodeUser()
  );
}


unpackTo(_o: StatusV1T): void {
  _o.executionId = this.executionId();
  _o.status = this.status();
  _o.proof = this.bb!.createScalarList<number>(this.proof.bind(this), this.proofLength());
  _o.executionDigest = this.bb!.createScalarList<number>(this.executionDigest.bind(this), this.executionDigestLength());
  _o.inputDigest = this.bb!.createScalarList<number>(this.inputDigest.bind(this), this.inputDigestLength());
  _o.committedOutputs = this.bb!.createScalarList<number>(this.committedOutputs.bind(this), this.committedOutputsLength());
  _o.assumptionDigest = this.bb!.createScalarList<number>(this.assumptionDigest.bind(this), this.assumptionDigestLength());
  _o.exitCodeSystem = this.exitCodeSystem();
  _o.exitCodeUser = this.exitCodeUser();
}
}

export class StatusV1T implements flatbuffers.IGeneratedObject {
constructor(
  public executionId: string|Uint8Array|null = null,
  public status: StatusTypes = StatusTypes.Unknown,
  public proof: (number)[] = [],
  public executionDigest: (number)[] = [],
  public inputDigest: (number)[] = [],
  public committedOutputs: (number)[] = [],
  public assumptionDigest: (number)[] = [],
  public exitCodeSystem: number = 0,
  public exitCodeUser: number = 0
){}


pack(builder:flatbuffers.Builder): flatbuffers.Offset {
  const executionId = (this.executionId !== null ? builder.createString(this.executionId!) : 0);
  const proof = StatusV1.createProofVector(builder, this.proof);
  const executionDigest = StatusV1.createExecutionDigestVector(builder, this.executionDigest);
  const inputDigest = StatusV1.createInputDigestVector(builder, this.inputDigest);
  const committedOutputs = StatusV1.createCommittedOutputsVector(builder, this.committedOutputs);
  const assumptionDigest = StatusV1.createAssumptionDigestVector(builder, this.assumptionDigest);

  return StatusV1.createStatusV1(builder,
    executionId,
    this.status,
    proof,
    executionDigest,
    inputDigest,
    committedOutputs,
    assumptionDigest,
    this.exitCodeSystem,
    this.exitCodeUser
  );
}
}
