require 'ffi'

module Test
  extend FFI::Library
  ffi_lib 'rusterr5'

  attach_function :parse_hex,
    %i[pointer pointer pointer],
    :uint32
  attach_function :cfm_err_get_msg,
    %i[pointer pointer],
    :uint32
  attach_function :cfm_err_get_code,
    %i[pointer pointer],
    :uint32
  attach_function :cfm_err_get_backtrace,
    %i[pointer pointer],
    :uint32
  attach_function :cfm_err_destroy,
    %i[pointer],
    :uint32
end

exit 1 if ARGV.empty?

ARGV.each do |str|
  puts "Processing '#{str}'"

  presult = FFI::MemoryPointer.new(:uint32)
  perr = FFI::MemoryPointer.new(:pointer)
  code = Test.parse_hex(str, presult, perr)
  if code != 0
    puts "Error #{code}"
    perr = perr.read_pointer
    pcode = FFI::MemoryPointer.new(:uint32)
    Test.cfm_err_get_code(perr, pcode)
    raise if pcode.read(:uint32) != code
    pmsg = FFI::MemoryPointer.new(:pointer)
    Test.cfm_err_get_msg(perr, pmsg)
    pmsg = pmsg.read_pointer
    puts "Message: #{pmsg.read_string}"
    pbt = FFI::MemoryPointer.new(:pointer)
    Test.cfm_err_get_backtrace(perr, pbt)
    pbt = pbt.read_pointer
    puts "Backtrace: #{pbt.read_string}"
    Test.cfm_err_destroy(perr)
  else
    puts "result: #{presult.read(:uint32)}"
  end
end

