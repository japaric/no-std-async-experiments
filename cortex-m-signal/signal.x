SECTIONS
{
  .signal 0 (INFO) : {
    *(.signal);
  }
}

ASSERT(SIZEOF(.signal) <= 32, "only 32 different signals are supported at the moment");
